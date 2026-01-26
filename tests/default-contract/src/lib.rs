// Find all our documentation at https://docs.near.org
use near_sdk::json_types::U64;
use near_sdk::{env, near, require, store, AccountId, NearToken, PanicOnDefault, Promise};

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct Bid {
    pub bidder: AccountId,
    pub bid: NearToken,
    pub bid_time: U64,
    pub bid_block_height: U64,
    pub bid_block_timestamp: U64,
    pub bid_epoch_height: U64,
    pub premium: bool,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    highest_bid: Bid,
    auction_end_time: U64,
    auctioneer: AccountId,
    claimed: bool,
    vector: Vec<u8>,
    sdk_vector: store::Vector<u8>,
    sdk_iterable_map: store::IterableMap<u8, u8>,
}

#[near]
impl Contract {
    #[init]
    #[private] // only callable by the contract's account
    pub fn init(end_time: U64, auctioneer: AccountId) -> Self {
        Self {
            highest_bid: Bid {
                bidder: env::current_account_id(),
                bid: NearToken::from_yoctonear(1),
                bid_time: U64::from(env::block_timestamp()),
                bid_block_height: U64::from(env::block_height()),
                bid_block_timestamp: U64::from(env::block_timestamp()),
                bid_epoch_height: U64::from(env::epoch_height()),
                premium: false,
            },
            auction_end_time: end_time,
            claimed: false,
            auctioneer,
            vector: vec![],
            sdk_vector: store::Vector::new(b"s"),
            sdk_iterable_map: store::IterableMap::new(b"i"),
        }
    }

    #[payable]
    pub fn bid(&mut self) -> Promise {
        // Assert the auction is still ongoing
        require!(
            env::block_timestamp() < self.auction_end_time.into(),
            "Auction has ended"
        );

        // Current bid
        let bid = env::attached_deposit();
        let bidder = env::predecessor_account_id();

        // Last bid
        let Bid {
            bidder: last_bidder,
            bid: last_bid,
            bid_time: _last_bid_time,
            bid_block_height: _last_bid_block_height,
            bid_block_timestamp: _last_bid_block_timestamp,
            bid_epoch_height: _last_bid_epoch_height,
            premium: _last_premium,
        } = self.highest_bid.clone();

        // Check if the deposit is higher than the current bid
        require!(bid > last_bid, "You must place a higher bid");

        // Update the highest bid
        self.highest_bid = Bid {
            bidder,
            bid,
            bid_time: U64::from(env::block_timestamp()),
            bid_block_height: U64::from(env::block_height()),
            bid_block_timestamp: U64::from(env::block_timestamp()),
            bid_epoch_height: U64::from(env::epoch_height()),
            premium: false,
        };

        // Transfer tokens back to the last bidder
        Promise::new(last_bidder).transfer(last_bid)
    }

    pub fn claim(&mut self) -> Promise {
        require!(
            env::block_timestamp() > self.auction_end_time.into(),
            "Auction has not ended yet"
        );

        require!(!self.claimed, "Auction has already been claimed");
        self.claimed = true;

        // Transfer tokens to the auctioneer
        Promise::new(self.auctioneer.clone()).transfer(self.highest_bid.bid)
    }

    pub fn fill_vector(&mut self) {
        for i in 0..1000 {
            self.vector.push(i as u8);
        }
    }

    pub fn fill_sdk_vector(&mut self) {
        for i in 0..1000 {
            self.sdk_vector.push(i as u8);
        }
    }
    pub fn fill_sdk_iterable_map(&mut self) {
        for i in 0..1000 {
            self.sdk_iterable_map.insert(i as u8, i as u8);
        }
    }

    pub fn get_vector(&self) -> Vec<u8> {
        self.vector.clone()
    }

    pub fn get_sdk_vector(&self) -> Vec<u8> {
        self.sdk_vector.iter().cloned().collect::<Vec<u8>>()
    }

    pub fn get_highest_bid(&self) -> Bid {
        self.highest_bid.clone()
    }

    pub fn get_auction_end_time(&self) -> U64 {
        self.auction_end_time
    }

    pub fn get_auctioneer(&self) -> AccountId {
        self.auctioneer.clone()
    }

    pub fn get_claimed(&self) -> bool {
        self.claimed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_contract() {
        let end_time: U64 = U64::from(1000);
        let alice: AccountId = "alice.near".parse().unwrap();
        let contract = Contract::init(end_time.clone(), alice.clone());

        let default_bid = contract.get_highest_bid();
        assert_eq!(default_bid.bidder, env::current_account_id());
        assert_eq!(default_bid.bid, NearToken::from_yoctonear(1));

        let auction_end_time = contract.get_auction_end_time();
        assert_eq!(auction_end_time, end_time);

        let auctioneer = contract.get_auctioneer();
        assert_eq!(auctioneer, alice);

        let claimed = contract.get_claimed();
        assert_eq!(claimed, false);
    }
}
