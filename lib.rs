#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(clippy::cast_possible_truncation)]

use ink::prelude::vec::Vec;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;


#[ink::contract]
mod game_lobby {
    use super::*;
    use ink::storage::traits::StorageLayout;

    #[ink(storage)]
    pub struct GameLobby {
        owner: AccountId,
        family_id: u32, // games of similar type, i.e. poker games will have same family id to group them
        max_players: u8,
        players: Vec<AccountId>,
        state: LobbyState,
    }

    #[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug)]
    #[cfg_attr(feature = "std", derive(TypeInfo, StorageLayout))]
    pub enum LobbyState {
        Registering,
        InPlay,
        Finished,
    }

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(TypeInfo, StorageLayout))]
    pub enum Error {
        LobbyFull,
        LobbyNotOpen,
        PlayerAlreadyJoined,
        PlayerNotFound,
        NotOwner,
    }

    impl GameLobby {
        #[ink(constructor)]
        pub fn new(family_id: u32, max_players: u8) -> Self {
            // Caller is the address that is calling this constructor
            let caller = Self::env().caller();

            Self {
                owner: caller,
                family_id,
                max_players,
                players: Vec::new(),
                state: LobbyState::Registering,
            }
        }

        #[ink(message)]
        pub fn join(&mut self) -> Result<(), Error> {
            let caller = Self::env().caller();

            // Carry out series of checks before joining player:

            // Check if player already joined
            if self.players.contains(&caller) {
                return Err(Error::PlayerAlreadyJoined);
            }

            // Check if lobby is full
            if self.players.len() >= self.max_players as usize {
                return Err(Error::LobbyFull);
            }

            // Check if lobby is open for registration
            if self.state != LobbyState::Registering {
                return Err(Error::LobbyNotOpen);
            }

            // All is good
            self.players.push(caller);

            // Auto-transition to InPlay if lobby is full
            if self.players.len() == self.max_players as usize {
                self.state = LobbyState::InPlay;
            }

            Ok(())
        }

        #[ink(message)]
        pub fn leave(&mut self) -> Result<(), Error> {
            let caller = Self::env().caller();
            
            // Check if lobby is open for registration
            if self.state != LobbyState::Registering {
                return Err(Error::LobbyNotOpen);
            }
            
            // Find and remove player
            if let Some(index) = self.players.iter().position(|p| p == &caller) {
                self.players.swap_remove(index);
                Ok(())
            } else {
                Err(Error::PlayerNotFound)
            }
        }

        #[ink(message)]
        pub fn get_players(&self) -> Vec<AccountId> {
            self.players.clone()
        }

        #[ink(message)]
        pub fn get_state(&self) -> LobbyState {
            self.state
        }
    }



    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::{test, DefaultEnvironment};

        #[ink::test]
        fn test_new() {
            let accounts = test::default_accounts::<DefaultEnvironment>();

            // create new lobby with family id of and max 4 players.
            let lobby = GameLobby::new(1, 4);

            // Assert
            assert_eq!(lobby.owner, accounts.alice); // Alice, who is acc#1 is owner
            assert_eq!(lobby.family_id, 1); // check family ID stored
            assert_eq!(lobby.max_players, 4); // check max players
            assert_eq!(lobby.players.len(), 0); // should be empty
            assert_eq!(lobby.state, LobbyState::Registering); // should default to registering state
        }

        #[ink::test]
        fn test_join() {
            let mut lobby = GameLobby::new(1, 2); // Note: changed max_players to 2

            // owner join lobby
            let result = lobby.join();

            assert_eq!(result.is_ok(), true);                   // should be ok
            assert_eq!(lobby.players.len(), 1);                 // should be 1 player
            assert_eq!(lobby.state, LobbyState::Registering);   // should still be in registering state

            // Add a second player with a generated caller bob
            test::set_caller::<DefaultEnvironment>(test::default_accounts::<DefaultEnvironment>().bob);
            let result = lobby.join();

            assert_eq!(result.is_ok(), true);               // should be ok
            assert_eq!(lobby.players.len(), 2);             // should be 2 players
            assert_eq!(lobby.state, LobbyState::InPlay);    // should transition to In-Play state
        }

        #[ink::test]
        fn test_leave(){
            let mut lobby = GameLobby::new(1, 3);
            lobby.join();

            test::set_caller::<DefaultEnvironment>(test::default_accounts::<DefaultEnvironment>().bob);
            lobby.join();

            assert_eq!(lobby.players.len(), 2);

            let result = lobby.leave();

            assert_eq!(result.is_ok(), true);       // should be ok, no err
            assert_eq!(lobby.players.len(), 1);     // should be one player left

        }

        #[ink::test]
        fn test_get_players() {
            let mut lobby = GameLobby::new(1, 4);
            let accounts = test::default_accounts::<DefaultEnvironment>();

            // Alice joins (default caller)
            assert_eq!(lobby.join(), Ok(()));

            // Bob joins
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert_eq!(lobby.join(), Ok(()));

            // Charlie joins
            test::set_caller::<DefaultEnvironment>(accounts.charlie);
            assert_eq!(lobby.join(), Ok(()));

            // Fetch players
            let players = lobby.get_players();

            // Validate length and order
            assert_eq!(players.len(), 3);
            assert_eq!(players[0], accounts.alice);
            assert_eq!(players[1], accounts.bob);
            assert_eq!(players[2], accounts.charlie);

            // Bob leaves
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert_eq!(lobby.leave(), Ok(()));

            let players = lobby.get_players();

            // Should have 3 players now
            assert_eq!(players.len(), 2);

            // Bob should be gone
            assert!(!players.contains(&accounts.bob));

            // Alice and Charlie should still be there
            assert!(players.contains(&accounts.alice));
            assert!(players.contains(&accounts.charlie));
        }

        #[ink::test]
        fn test_join_fails_when_lobby_is_full() {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            let mut lobby = GameLobby::new(99, 2); // max_players = 2

            // Alice joins
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            assert_eq!(lobby.join(), Ok(()));

            // Bob joins
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert_eq!(lobby.join(), Ok(()));

            // Charlie tries to join (should fail)
            test::set_caller::<DefaultEnvironment>(accounts.charlie);
            let result = lobby.join();
            assert_eq!(result, Err(Error::LobbyFull));

            // Players list should remain 2
            let players = lobby.get_players();
            assert_eq!(players.len(), 2);
            assert!(players.contains(&accounts.alice));
            assert!(players.contains(&accounts.bob));
        }

        #[ink::test]
        fn test_join_fails_if_already_joined() {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            let mut lobby = GameLobby::new(1, 3);

            // Alice joins
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            assert_eq!(lobby.join(), Ok(()));

            // Alice tries to join again
            let result = lobby.join();
            assert_eq!(result, Err(Error::PlayerAlreadyJoined));

            // Only one entry should exist
            let players = lobby.get_players();
            assert_eq!(players.len(), 1);
            assert_eq!(players[0], accounts.alice);
        }


    }
}
