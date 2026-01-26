use std::vec;

use near_sdk::borsh::{self};
use near_sdk::env::{storage_read, storage_write};
use near_sdk::json_types::U64;
use near_sdk::{env, near, require, store, AccountId, NearToken, Promise};

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
#[derive(Default)]
pub struct Contract {
    // we don't want stuff here
}

#[near]
impl Contract {
    #[init]
    pub fn init(end_time: U64, auctioneer: AccountId) -> Self {
        let highest_bid = Bid {
            bidder: env::current_account_id(),
            bid: NearToken::from_yoctonear(1),
            bid_time: U64::from(env::block_timestamp()),
            bid_block_height: U64::from(env::block_height()),
            bid_block_timestamp: U64::from(env::block_timestamp()),
            bid_epoch_height: U64::from(env::epoch_height()),
            premium: false,
        };
        let vector: Vec<u8> = vec![];
        let sdk_vector: store::Vector<u8> = store::Vector::new(b"s");
        let sdk_iterable_map: store::IterableMap<u8, u8> = store::IterableMap::new(b"m");
        storage_write(b"highest_bid", &borsh::to_vec(&highest_bid).unwrap());
        storage_write(b"auction_end_time", &borsh::to_vec(&end_time).unwrap());
        storage_write(b"auctioneer", &borsh::to_vec(&auctioneer).unwrap());
        storage_write(b"claimed", &borsh::to_vec(&false).unwrap());
        storage_write(b"vector", &borsh::to_vec(&vector).unwrap());
        storage_write(b"s", &borsh::to_vec(&sdk_vector).unwrap());
        storage_write(b"i", &borsh::to_vec(&sdk_iterable_map).unwrap());

        Self {}
    }

    #[payable]
    pub fn bid(&mut self) -> Promise {
        // Assert the auction is still ongoing
        let auction_end_time: U64 =
            borsh::from_slice(&storage_read(b"auction_end_time").unwrap()).unwrap();
        require!(
            env::block_timestamp() < auction_end_time.0,
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
        } = borsh::from_slice(&storage_read(b"highest_bid").unwrap()).unwrap();

        // Check if the deposit is higher than the current bid
        require!(bid > last_bid, "You must place a higher bid");

        // Update the highest bid
        // self.highest_bid = Bid { bidder, bid };
        storage_write(
            b"highest_bid",
            &borsh::to_vec(&Bid {
                bidder,
                bid,
                bid_time: U64::from(env::block_timestamp()),
                bid_block_height: U64::from(env::block_height()),
                bid_block_timestamp: U64::from(env::block_timestamp()),
                bid_epoch_height: U64::from(env::epoch_height()),
                premium: false,
            })
            .unwrap(),
        );

        // Transfer tokens back to the last bidder
        Promise::new(last_bidder).transfer(last_bid)
    }

    pub fn claim(&mut self) -> Promise {
        let auction_end_time: U64 =
            borsh::from_slice(&storage_read(b"auction_end_time").unwrap()).unwrap();
        require!(
            env::block_timestamp() > auction_end_time.0,
            "Auction has not ended yet"
        );

        let claimed: bool = borsh::from_slice(&storage_read(b"claimed").unwrap()).unwrap();
        require!(!claimed, "Auction has already been claimed");
        // self.claimed = true;
        storage_write(b"claimed", &borsh::to_vec(&true).unwrap());

        // Transfer tokens to the auctioneer
        let auctioneer: AccountId =
            borsh::from_slice(&storage_read(b"auctioneer").unwrap()).unwrap();
        let highest_bid: Bid = borsh::from_slice(&storage_read(b"highest_bid").unwrap()).unwrap();
        Promise::new(auctioneer).transfer(highest_bid.bid)
    }

    pub fn fill_vector(&mut self) {
        let mut vector: Vec<u8> = borsh::from_slice(&storage_read(b"vector").unwrap()).unwrap();
        for i in 0..1000 {
            vector.push(i as u8);
        }
        storage_write(b"vector", &borsh::to_vec(&vector).unwrap());
    }

    pub fn fill_sdk_vector(&mut self) {
        let mut sdk_vector: store::Vector<u8> =
            borsh::from_slice(&storage_read(b"s").unwrap()).unwrap();
        for i in 0..1000 {
            sdk_vector.push(i as u8);
        }
        storage_write(b"s", &borsh::to_vec(&sdk_vector).unwrap());
    }

    pub fn fill_sdk_iterable_map(&mut self) {
        let mut sdk_iterable_map: store::IterableMap<u8, u8> =
            borsh::from_slice(&storage_read(b"i").unwrap()).unwrap();
        for i in 0..1000 {
            sdk_iterable_map.insert(i as u8, i as u8);
        }
        storage_write(b"i", &borsh::to_vec(&sdk_iterable_map).unwrap());
    }

    pub fn get_vector(&self) -> Vec<u8> {
        borsh::from_slice(&storage_read(b"vector").unwrap()).unwrap()
    }

    pub fn get_sdk_vector(&self) -> Vec<u8> {
        borsh::from_slice(&storage_read(b"a").unwrap()).unwrap()
    }

    pub fn get_highest_bid(&self) -> Bid {
        borsh::from_slice(&storage_read(b"highest_bid").unwrap()).unwrap()
    }

    pub fn get_auction_end_time(&self) -> U64 {
        borsh::from_slice(&storage_read(b"auction_end_time").unwrap()).unwrap()
    }

    pub fn get_auctioneer(&self) -> AccountId {
        borsh::from_slice(&storage_read(b"auctioneer").unwrap()).unwrap()
    }

    pub fn get_claimed(&self) -> bool {
        borsh::from_slice(&storage_read(b"claimed").unwrap()).unwrap()
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
