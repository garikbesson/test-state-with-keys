use near_api::{AccountId, NearGas, NearToken};
use near_sdk::serde_json::json;

#[derive(near_sdk::serde::Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Bid {
    pub bidder: AccountId,
    pub bid: NearToken,
}

#[tokio::test]
async fn test_contract_is_operational() -> testresult::TestResult<()> {
    let contract_wasm_path = cargo_near_build::build_with_cli(Default::default())?;
    let contract_wasm = std::fs::read(contract_wasm_path)?;

    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let sandbox_network =
        near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);

    // Create accounts
    let alice = create_subaccount(&sandbox, "alice.sandbox").await?;
    let bob = create_subaccount(&sandbox, "bob.sandbox").await?;
    let auctioneer = create_subaccount(&sandbox, "auctioneer.sandbox").await?;
    let contract = create_subaccount(&sandbox, "contract.sandbox")
        .await?
        .as_contract();

    // Deploy and initialize contract
    let signer = near_api::Signer::from_secret_key(
        near_sandbox::config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY
            .parse()
            .unwrap(),
    )?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)?
        .as_secs();
    let a_minute_from_now = (now + 60) * 1000000000;
    near_api::Contract::deploy(contract.account_id().clone())
        .use_code(contract_wasm)
        .with_init_call(
            "init",
            json!({"end_time": a_minute_from_now.to_string(), "auctioneer": auctioneer.account_id()}),
        )?
        .with_signer(signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();

    // Alice makes first bid
    let function = contract
        .call_function("bid", ())
        .transaction()
        .deposit(NearToken::from_near(1))
        .with_signer(alice.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();

    let highest_bid: Bid = contract
        .call_function("get_highest_bid", ())
        .read_only()
        .fetch_from(&sandbox_network)
        .await?
        .data;
    assert_eq!(highest_bid.bid, NearToken::from_near(1));
    assert_eq!(&highest_bid.bidder, alice.account_id());

    let alice_balance = alice
        .tokens()
        .near_balance()
        .fetch_from(&sandbox_network)
        .await?
        .total;

    // Bob makes a higher bid
    contract
        .call_function("bid", ())
        .transaction()
        .deposit(NearToken::from_near(2))
        .with_signer(bob.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();

    let highest_bid: Bid = contract
        .call_function("get_highest_bid", ())
        .read_only()
        .fetch_from(&sandbox_network)
        .await?
        .data;
    assert_eq!(highest_bid.bid, NearToken::from_near(2));
    assert_eq!(&highest_bid.bidder, bob.account_id());

    // Check that Alice was refunded
    let new_alice_balance = alice
        .tokens()
        .near_balance()
        .fetch_from(&sandbox_network)
        .await?
        .total;

    assert!(new_alice_balance == alice_balance.saturating_add(NearToken::from_near(1)));

    // Alice tries to make a bid with less NEAR than the previous
    contract
        .call_function("bid", ())
        .transaction()
        .deposit(NearToken::from_near(1))
        .with_signer(alice.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_failure();

    // Auctioneer claims auction but did not finish
    contract
        .call_function("claim", ())
        .transaction()
        .gas(NearGas::from_tgas(30))
        .with_signer(auctioneer.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_failure();

    // Fast forward 200 blocks
    let blocks_to_advance = 200;
    sandbox.fast_forward(blocks_to_advance).await?;

    // Auctioneer claims the auction
    contract
        .call_function("claim", ())
        .transaction()
        .gas(NearGas::from_tgas(30))
        .with_signer(auctioneer.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();

    // Checks the auctioneer has the correct balance
    let auctioneer_balance = auctioneer
        .tokens()
        .near_balance()
        .fetch_from(&sandbox_network)
        .await?
        .total;
    assert!(auctioneer_balance <= NearToken::from_near(12));
    assert!(auctioneer_balance > NearToken::from_millinear(11990));

    // Auctioneer tries to claim the auction again
    contract
        .call_function("claim", ())
        .transaction()
        .gas(NearGas::from_tgas(30))
        .with_signer(auctioneer.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_failure();

    // Alice tries to make a bid when the auction is over
    contract
        .call_function("bid", ())
        .transaction()
        .deposit(NearToken::from_near(1))
        .with_signer(alice.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_failure();

    Ok(())
}

#[tokio::test]
async fn test_difference_between_contracts() -> testresult::TestResult<()> {
    // Build our custom state contract
    let contract_wasm_path = cargo_near_build::build_with_cli(Default::default())?;
    let contract_wasm = std::fs::read(contract_wasm_path)?;

    // Build default contract
    let default_contract_wasm_path = cargo_near_build::build_with_cli(
        cargo_near_build::BuildOpts::builder()
            .manifest_path("tests/default-contract/Cargo.toml")
            .build(),
    )?;
    let default_contract_wasm = std::fs::read(default_contract_wasm_path)?;

    // Initialize sandbox
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let sandbox_network =
        near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);

    // Create accounts
    let alice = create_subaccount(&sandbox, "alice.sandbox").await?;
    let auctioneer = create_subaccount(&sandbox, "auctioneer.sandbox").await?;
    let contract_account = create_subaccount(&sandbox, "contract.sandbox").await?;
    let contract = contract_account.as_contract();
    let default_contract_account = create_subaccount(&sandbox, "default_contract.sandbox").await?;
    let default_contract = default_contract_account.as_contract();

    // Iinitialize parameters for the contracts
    let signer = near_api::Signer::from_secret_key(
        near_sandbox::config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY
            .parse()
            .unwrap(),
    )?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)?
        .as_secs();
    let a_minute_from_now = (now + 60) * 1000000000;

    // Deploy our custom state contract with init call
    let deploy_contract_result = near_api::Contract::deploy(contract.account_id().clone())
        .use_code(contract_wasm.clone())
        .with_init_call(
            "init",
            json!({"end_time": a_minute_from_now.to_string(), "auctioneer": auctioneer.account_id()}),
        )?
        .with_signer(signer.clone())
        .send_to(&sandbox_network)
        .await?;
    println!(
        "deploy_result_gas: {:?} Ggas",
        deploy_contract_result.total_gas_burnt.as_ggas()
    );
    assert!(deploy_contract_result.is_success());

    // Deploy default contract with init call
    let deploy_default_contract_result = near_api::Contract::deploy(
        default_contract.account_id().clone(),
    )
    .use_code(default_contract_wasm.clone())
    .with_init_call(
        "init",
        json!({"end_time": a_minute_from_now.to_string(), "auctioneer": auctioneer.account_id()}),
    )?
    .with_signer(signer.clone())
    .send_to(&sandbox_network)
    .await?;
    println!(
        "deploy_default_contract_result_gas: {:?} Ggas\n",
        deploy_default_contract_result.total_gas_burnt.as_ggas()
    );
    assert!(deploy_default_contract_result.is_success());

    let deploy_gas_difference = deploy_contract_result
        .total_gas_burnt
        .saturating_sub(deploy_default_contract_result.total_gas_burnt);
    println!(
        "deploy_gas_difference: {:?} Ggas",
        deploy_gas_difference.as_ggas()
    );
    let deploy_gas_difference_percentage = deploy_gas_difference
        .saturating_div(deploy_default_contract_result.total_gas_burnt.as_gas())
        .saturating_mul(100);
    println!(
        "deploy_gas_difference_percentage: {:?}%\n",
        deploy_gas_difference_percentage.as_gas()
    );

    // Get the storage locked for our custom state contract
    let contract_storage_locked = contract_account
        .tokens()
        .near_balance()
        .fetch_from(&sandbox_network)
        .await?
        .storage_locked;
    println!(
        "contract_storage_locked: {:?}",
        contract_storage_locked.as_millinear()
    );

    // Get the storage locked for default contract
    let default_contract_storage_locked = default_contract_account
        .tokens()
        .near_balance()
        .fetch_from(&sandbox_network)
        .await?
        .storage_locked;
    println!(
        "default_contract_storage_locked: {:?}\n",
        default_contract_storage_locked.as_millinear()
    );

    // Alice makes first bid in our custom state contract
    let bid_contract_result = contract
        .call_function("bid", ())
        .transaction()
        .deposit(NearToken::from_near(1))
        .with_signer(alice.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();
    println!(
        "bid_contract_result_gas: {:?} Ggas",
        bid_contract_result.total_gas_burnt.as_ggas()
    );

    // Alice makes first bid in default contract
    let bid_default_contract_result = default_contract
        .call_function("bid", ())
        .transaction()
        .deposit(NearToken::from_near(1))
        .with_signer(alice.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();
    println!(
        "bid_default_contract_result_gas: {:?} Ggas\n",
        bid_default_contract_result.total_gas_burnt.as_ggas()
    );

    // Fast forward 200 blocks
    let blocks_to_advance = 200;
    sandbox.fast_forward(blocks_to_advance).await?;

    // Auctioneer claims the auction in our custom state contract
    let claim_contract_result = contract
        .call_function("claim", ())
        .transaction()
        .gas(NearGas::from_tgas(30))
        .with_signer(auctioneer.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();
    println!(
        "claim_contract_result_gas: {:?} Ggas",
        claim_contract_result.total_gas_burnt.as_ggas()
    );

    // Auctioneer claims the auction in default contract
    let claim_default_contract_result = default_contract
        .call_function("claim", ())
        .transaction()
        .gas(NearGas::from_tgas(30))
        .with_signer(auctioneer.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();
    println!(
        "claim_default_contract_result_gas: {:?} Ggas\n",
        claim_default_contract_result.total_gas_burnt.as_ggas()
    );

    // Fill vector in our custom state contract
    let fill_vector_contract_result = contract
        .call_function("fill_vector", ())
        .transaction()
        .gas(NearGas::from_tgas(30))
        .with_signer(auctioneer.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();
    println!(
        "fill_vector_contract_result_gas: {:?} Ggas",
        fill_vector_contract_result.total_gas_burnt.as_ggas()
    );

    // Fill vector in default contract
    let fill_vector_default_contract_result = default_contract
        .call_function("fill_vector", ())
        .transaction()
        .gas(NearGas::from_tgas(30))
        .with_signer(auctioneer.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();
    println!(
        "fill_vector_default_contract_result_gas: {:?} Ggas\n",
        fill_vector_default_contract_result
            .total_gas_burnt
            .as_ggas()
    );

    // Fill sdk vector in our custom state contract
    let fill_sdk_vector_contract_result = contract
        .call_function("fill_sdk_vector", ())
        .transaction()
        .gas(NearGas::from_tgas(300))
        .with_signer(auctioneer.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();
    println!(
        "fill_sdk_vector_contract_result_gas: {:?} Ggas",
        fill_sdk_vector_contract_result.total_gas_burnt.as_ggas()
    );

    // Fill sdk vector in default contract
    let fill_sdk_vector_default_contract_result = default_contract
        .call_function("fill_sdk_vector", ())
        .transaction()
        .gas(NearGas::from_tgas(300))
        .with_signer(auctioneer.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();
    println!(
        "fill_sdk_vector_default_contract_result_gas: {:?} Ggas\n",
        fill_sdk_vector_default_contract_result
            .total_gas_burnt
            .as_ggas()
    );

    // Fill sdk iterable map in our custom state contract
    let fill_sdk_iterable_map_contract_result = contract
        .call_function("fill_sdk_iterable_map", ())
        .transaction()
        .gas(NearGas::from_tgas(300))
        .with_signer(auctioneer.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();
    println!(
        "fill_sdk_iterable_map_contract_result_gas: {:?} Ggas",
        fill_sdk_iterable_map_contract_result
            .total_gas_burnt
            .as_ggas()
    );

    // Fill sdk iterable map in default contract
    let fill_sdk_iterable_map_default_contract_result = default_contract
        .call_function("fill_sdk_iterable_map", ())
        .transaction()
        .gas(NearGas::from_tgas(300))
        .with_signer(auctioneer.account_id().clone(), signer.clone())
        .send_to(&sandbox_network)
        .await?
        .assert_success();
    println!(
        "fill_sdk_iterable_map_default_contract_result_gas: {:?} Ggas\n",
        fill_sdk_iterable_map_default_contract_result
            .total_gas_burnt
            .as_ggas()
    );

    // Check that our custom state contract is more expensive to call methods than the default contract
    assert!(
        (deploy_contract_result.total_gas_burnt > deploy_default_contract_result.total_gas_burnt)
            && (contract_storage_locked > default_contract_storage_locked)
            && (bid_contract_result.total_gas_burnt > bid_default_contract_result.total_gas_burnt)
            && (claim_contract_result.total_gas_burnt
                > claim_default_contract_result.total_gas_burnt)
            && (fill_vector_contract_result.total_gas_burnt
                > fill_vector_default_contract_result.total_gas_burnt)
            && (fill_sdk_vector_contract_result.total_gas_burnt
                > fill_sdk_vector_default_contract_result.total_gas_burnt)
            && (fill_sdk_iterable_map_contract_result.total_gas_burnt
                > fill_sdk_iterable_map_default_contract_result.total_gas_burnt)
    );

    Ok(())
}

async fn create_subaccount(
    sandbox: &near_sandbox::Sandbox,
    name: &str,
) -> testresult::TestResult<near_api::Account> {
    let account_id: AccountId = name.parse().unwrap();
    sandbox
        .create_account(account_id.clone())
        .initial_balance(NearToken::from_near(10))
        .send()
        .await?;
    Ok(near_api::Account(account_id))
}
