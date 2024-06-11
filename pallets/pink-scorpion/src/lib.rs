#![cfg_attr(not(feature = "std"), no_std)]

#[frame_support::pallet]
pub mod pallet_pink_scorpion {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use scale_info::prelude::vec::Vec;
    use core::convert::TryInto;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Clone, PartialEq, Eq)]
    pub struct FSEvent {
        pub creationtime: [u8; 64],
        pub filepath: [u8; 256],
        pub eventkey: [u8; 128],
    }

    #[pallet::storage]
    #[pallet::getter(fn info)]
    pub(super) type DisAssembly<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, Vec<(u64, FSEvent)>, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// An event indicating a file has been disassembled.
        FileDisassembled { who: T::AccountId, event: FSEvent },
        /// An event indicating a file has been reassembled.
        FileReassembled { who: T::AccountId, creation_time: Vec<u8>, file_path: Vec<u8>, event_key: Vec<u8> },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Error indicating the creation time is too long.
        CreationTimeTooLong,
        /// Error indicating the file path is too long.
        FilePathTooLong,
        /// Error indicating the event key is too long.
        EventKeyTooLong,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        pub fn file_disassembled(
            origin: OriginFor<T>,
            creation_time: Vec<u8>,
            file_path: Vec<u8>,
            event_key: Vec<u8>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // Validate input lengths
            ensure!(creation_time.len() <= 64, Error::<T>::CreationTimeTooLong);
            ensure!(file_path.len() <= 256, Error::<T>::FilePathTooLong);
            ensure!(event_key.len() <= 128, Error::<T>::EventKeyTooLong);

            // Create FSEvent instance
            let event = FSEvent {
                creationtime: Self::vec_to_array::<64>(creation_time)?,
                filepath: Self::vec_to_array::<256>(file_path)?,
                eventkey: Self::vec_to_array::<128>(event_key)?,
            };

            // Get the current events for the sender
            let mut events_vec = DisAssembly::<T>::get(&sender).unwrap_or_default();

            // Generate a new key for the event using the block number converted to u64
            let key: u64 = frame_system::Pallet::<T>::block_number().try_into().map_err(|_| Error::<T>::FilePathTooLong)?;

            // Insert the new event into the Vec
            events_vec.push((key, event.clone()));

            // Store the updated events in storage
            DisAssembly::<T>::insert(&sender, events_vec);

            // Emit event for file disassembly
            Self::deposit_event(Event::<T>::FileDisassembled {
                who: sender,
                event,
            });

            Ok(())
        }

        pub fn file_reassembled(
            origin: OriginFor<T>,
            creation_time: Vec<u8>,
            file_path: Vec<u8>,
            event_key: Vec<u8>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // Emit event for file reassembly
            Self::deposit_event(Event::FileReassembled { who: sender, creation_time, file_path, event_key });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn vec_to_array<const N: usize>(vec: Vec<u8>) -> Result<[u8; N], Error<T>> {
            let mut array = [0u8; N];
            if vec.len() > N {
                return Err(Error::<T>::CreationTimeTooLong);
            }
            array[..vec.len()].copy_from_slice(&vec);
            Ok(array)
        }
    }
}
