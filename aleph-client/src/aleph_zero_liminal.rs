#[allow(dead_code, unused_imports, non_camel_case_types)]
pub mod api {
    use super::api as root_mod;
    pub static PALLETS: [&str; 22usize] = [
        "System",
        "RandomnessCollectiveFlip",
        "Scheduler",
        "Aura",
        "Timestamp",
        "Balances",
        "TransactionPayment",
        "Authorship",
        "Staking",
        "History",
        "Session",
        "Aleph",
        "Elections",
        "Treasury",
        "Vesting",
        "Utility",
        "Multisig",
        "Sudo",
        "Contracts",
        "NominationPools",
        "Identity",
        "BabyLiminal",
    ];
    #[derive(
        :: subxt :: ext :: codec :: Decode,
        :: subxt :: ext :: codec :: Encode,
        Clone,
        Debug,
        Eq,
        PartialEq,
    )]
    pub enum Event {
        #[codec(index = 0)]
        System(system::Event),
        #[codec(index = 2)]
        Scheduler(scheduler::Event),
        #[codec(index = 5)]
        Balances(balances::Event),
        #[codec(index = 6)]
        TransactionPayment(transaction_payment::Event),
        #[codec(index = 8)]
        Staking(staking::Event),
        #[codec(index = 10)]
        Session(session::Event),
        #[codec(index = 11)]
        Aleph(aleph::Event),
        #[codec(index = 12)]
        Elections(elections::Event),
        #[codec(index = 13)]
        Treasury(treasury::Event),
        #[codec(index = 14)]
        Vesting(vesting::Event),
        #[codec(index = 15)]
        Utility(utility::Event),
        #[codec(index = 16)]
        Multisig(multisig::Event),
        #[codec(index = 17)]
        Sudo(sudo::Event),
        #[codec(index = 18)]
        Contracts(contracts::Event),
        #[codec(index = 19)]
        NominationPools(nomination_pools::Event),
        #[codec(index = 20)]
        Identity(identity::Event),
        #[codec(index = 21)]
        BabyLiminal(baby_liminal::Event),
    }
    pub mod system {
        use super::{root_mod, runtime_types};
        #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Remark {
                pub remark: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                :: subxt :: ext :: codec :: CompactAs,
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetHeapPages {
                pub pages: ::core::primitive::u64,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetCode {
                pub code: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetCodeWithoutChecks {
                pub code: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetStorage {
                pub items: ::std::vec::Vec<(
                    ::std::vec::Vec<::core::primitive::u8>,
                    ::std::vec::Vec<::core::primitive::u8>,
                )>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct KillStorage {
                pub keys: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct KillPrefix {
                pub prefix: ::std::vec::Vec<::core::primitive::u8>,
                pub subkeys: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct RemarkWithEvent {
                pub remark: ::std::vec::Vec<::core::primitive::u8>,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                #[doc = "Make some on-chain remark."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(1)`"]
                #[doc = "# </weight>"]
                pub fn remark(
                    &self,
                    remark: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::StaticTxPayload<Remark> {
                    ::subxt::tx::StaticTxPayload::new(
                        "System",
                        "remark",
                        Remark { remark },
                        [
                            101u8, 80u8, 195u8, 226u8, 224u8, 247u8, 60u8, 128u8, 3u8, 101u8, 51u8,
                            147u8, 96u8, 126u8, 76u8, 230u8, 194u8, 227u8, 191u8, 73u8, 160u8,
                            146u8, 87u8, 147u8, 243u8, 28u8, 228u8, 116u8, 224u8, 181u8, 129u8,
                            160u8,
                        ],
                    )
                }
                #[doc = "Set the number of pages in the WebAssembly environment's heap."]
                pub fn set_heap_pages(
                    &self,
                    pages: ::core::primitive::u64,
                ) -> ::subxt::tx::StaticTxPayload<SetHeapPages> {
                    ::subxt::tx::StaticTxPayload::new(
                        "System",
                        "set_heap_pages",
                        SetHeapPages { pages },
                        [
                            43u8, 103u8, 128u8, 49u8, 156u8, 136u8, 11u8, 204u8, 80u8, 6u8, 244u8,
                            86u8, 171u8, 44u8, 140u8, 225u8, 142u8, 198u8, 43u8, 87u8, 26u8, 45u8,
                            125u8, 222u8, 165u8, 254u8, 172u8, 158u8, 39u8, 178u8, 86u8, 87u8,
                        ],
                    )
                }
                #[doc = "Set the new runtime code."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(C + S)` where `C` length of `code` and `S` complexity of `can_set_code`"]
                #[doc = "- 1 call to `can_set_code`: `O(S)` (calls `sp_io::misc::runtime_version` which is"]
                #[doc = "  expensive)."]
                #[doc = "- 1 storage write (codec `O(C)`)."]
                #[doc = "- 1 digest item."]
                #[doc = "- 1 event."]
                #[doc = "The weight of this function is dependent on the runtime, but generally this is very"]
                #[doc = "expensive. We will treat this as a full block."]
                #[doc = "# </weight>"]
                pub fn set_code(
                    &self,
                    code: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::StaticTxPayload<SetCode> {
                    ::subxt::tx::StaticTxPayload::new(
                        "System",
                        "set_code",
                        SetCode { code },
                        [
                            27u8, 104u8, 244u8, 205u8, 188u8, 254u8, 121u8, 13u8, 106u8, 120u8,
                            244u8, 108u8, 97u8, 84u8, 100u8, 68u8, 26u8, 69u8, 93u8, 128u8, 107u8,
                            4u8, 3u8, 142u8, 13u8, 134u8, 196u8, 62u8, 113u8, 181u8, 14u8, 40u8,
                        ],
                    )
                }
                #[doc = "Set the new runtime code without doing any checks of the given `code`."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(C)` where `C` length of `code`"]
                #[doc = "- 1 storage write (codec `O(C)`)."]
                #[doc = "- 1 digest item."]
                #[doc = "- 1 event."]
                #[doc = "The weight of this function is dependent on the runtime. We will treat this as a full"]
                #[doc = "block. # </weight>"]
                pub fn set_code_without_checks(
                    &self,
                    code: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::StaticTxPayload<SetCodeWithoutChecks> {
                    ::subxt::tx::StaticTxPayload::new(
                        "System",
                        "set_code_without_checks",
                        SetCodeWithoutChecks { code },
                        [
                            102u8, 160u8, 125u8, 235u8, 30u8, 23u8, 45u8, 239u8, 112u8, 148u8,
                            159u8, 158u8, 42u8, 93u8, 206u8, 94u8, 80u8, 250u8, 66u8, 195u8, 60u8,
                            40u8, 142u8, 169u8, 183u8, 80u8, 80u8, 96u8, 3u8, 231u8, 99u8, 216u8,
                        ],
                    )
                }
                #[doc = "Set some items of storage."]
                pub fn set_storage(
                    &self,
                    items: ::std::vec::Vec<(
                        ::std::vec::Vec<::core::primitive::u8>,
                        ::std::vec::Vec<::core::primitive::u8>,
                    )>,
                ) -> ::subxt::tx::StaticTxPayload<SetStorage> {
                    ::subxt::tx::StaticTxPayload::new(
                        "System",
                        "set_storage",
                        SetStorage { items },
                        [
                            74u8, 43u8, 106u8, 255u8, 50u8, 151u8, 192u8, 155u8, 14u8, 90u8, 19u8,
                            45u8, 165u8, 16u8, 235u8, 242u8, 21u8, 131u8, 33u8, 172u8, 119u8, 78u8,
                            140u8, 10u8, 107u8, 202u8, 122u8, 235u8, 181u8, 191u8, 22u8, 116u8,
                        ],
                    )
                }
                #[doc = "Kill some items from storage."]
                pub fn kill_storage(
                    &self,
                    keys: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
                ) -> ::subxt::tx::StaticTxPayload<KillStorage> {
                    ::subxt::tx::StaticTxPayload::new(
                        "System",
                        "kill_storage",
                        KillStorage { keys },
                        [
                            174u8, 174u8, 13u8, 174u8, 75u8, 138u8, 128u8, 235u8, 222u8, 216u8,
                            85u8, 18u8, 198u8, 1u8, 138u8, 70u8, 19u8, 108u8, 209u8, 41u8, 228u8,
                            67u8, 130u8, 230u8, 160u8, 207u8, 11u8, 180u8, 139u8, 242u8, 41u8,
                            15u8,
                        ],
                    )
                }
                #[doc = "Kill all storage items with a key that starts with the given prefix."]
                #[doc = ""]
                #[doc = "**NOTE:** We rely on the Root origin to provide us the number of subkeys under"]
                #[doc = "the prefix we are removing to accurately calculate the weight of this function."]
                pub fn kill_prefix(
                    &self,
                    prefix: ::std::vec::Vec<::core::primitive::u8>,
                    subkeys: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<KillPrefix> {
                    ::subxt::tx::StaticTxPayload::new(
                        "System",
                        "kill_prefix",
                        KillPrefix { prefix, subkeys },
                        [
                            203u8, 116u8, 217u8, 42u8, 154u8, 215u8, 77u8, 217u8, 13u8, 22u8,
                            193u8, 2u8, 128u8, 115u8, 179u8, 115u8, 187u8, 218u8, 129u8, 34u8,
                            80u8, 4u8, 173u8, 120u8, 92u8, 35u8, 237u8, 112u8, 201u8, 207u8, 200u8,
                            48u8,
                        ],
                    )
                }
                #[doc = "Make some on-chain remark and emit event."]
                pub fn remark_with_event(
                    &self,
                    remark: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::StaticTxPayload<RemarkWithEvent> {
                    ::subxt::tx::StaticTxPayload::new(
                        "System",
                        "remark_with_event",
                        RemarkWithEvent { remark },
                        [
                            123u8, 225u8, 180u8, 179u8, 144u8, 74u8, 27u8, 85u8, 101u8, 75u8,
                            134u8, 44u8, 181u8, 25u8, 183u8, 158u8, 14u8, 213u8, 56u8, 225u8,
                            136u8, 88u8, 26u8, 114u8, 178u8, 43u8, 176u8, 43u8, 240u8, 84u8, 116u8,
                            46u8,
                        ],
                    )
                }
            }
        }
        #[doc = "Event for the System pallet."]
        pub type Event = runtime_types::frame_system::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "An extrinsic completed successfully."]
            pub struct ExtrinsicSuccess {
                pub dispatch_info: runtime_types::frame_support::dispatch::DispatchInfo,
            }
            impl ::subxt::events::StaticEvent for ExtrinsicSuccess {
                const PALLET: &'static str = "System";
                const EVENT: &'static str = "ExtrinsicSuccess";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "An extrinsic failed."]
            pub struct ExtrinsicFailed {
                pub dispatch_error: runtime_types::sp_runtime::DispatchError,
                pub dispatch_info: runtime_types::frame_support::dispatch::DispatchInfo,
            }
            impl ::subxt::events::StaticEvent for ExtrinsicFailed {
                const PALLET: &'static str = "System";
                const EVENT: &'static str = "ExtrinsicFailed";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "`:code` was updated."]
            pub struct CodeUpdated;
            impl ::subxt::events::StaticEvent for CodeUpdated {
                const PALLET: &'static str = "System";
                const EVENT: &'static str = "CodeUpdated";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A new account was created."]
            pub struct NewAccount {
                pub account: ::subxt::ext::sp_core::crypto::AccountId32,
            }
            impl ::subxt::events::StaticEvent for NewAccount {
                const PALLET: &'static str = "System";
                const EVENT: &'static str = "NewAccount";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "An account was reaped."]
            pub struct KilledAccount {
                pub account: ::subxt::ext::sp_core::crypto::AccountId32,
            }
            impl ::subxt::events::StaticEvent for KilledAccount {
                const PALLET: &'static str = "System";
                const EVENT: &'static str = "KilledAccount";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "On on-chain remark happened."]
            pub struct Remarked {
                pub sender: ::subxt::ext::sp_core::crypto::AccountId32,
                pub hash: ::subxt::ext::sp_core::H256,
            }
            impl ::subxt::events::StaticEvent for Remarked {
                const PALLET: &'static str = "System";
                const EVENT: &'static str = "Remarked";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                #[doc = " The full account information for a particular account ID."]
                pub fn account(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::frame_system::AccountInfo<
                            ::core::primitive::u32,
                            runtime_types::pallet_balances::AccountData<::core::primitive::u128>,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "Account",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Blake2_128Concat,
                        )],
                        [
                            176u8, 187u8, 21u8, 220u8, 159u8, 204u8, 127u8, 14u8, 21u8, 69u8, 77u8,
                            114u8, 230u8, 141u8, 107u8, 79u8, 23u8, 16u8, 174u8, 243u8, 252u8,
                            42u8, 65u8, 120u8, 229u8, 38u8, 210u8, 255u8, 22u8, 40u8, 109u8, 223u8,
                        ],
                    )
                }
                #[doc = " The full account information for a particular account ID."]
                pub fn account_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::frame_system::AccountInfo<
                            ::core::primitive::u32,
                            runtime_types::pallet_balances::AccountData<::core::primitive::u128>,
                        >,
                    >,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "Account",
                        Vec::new(),
                        [
                            176u8, 187u8, 21u8, 220u8, 159u8, 204u8, 127u8, 14u8, 21u8, 69u8, 77u8,
                            114u8, 230u8, 141u8, 107u8, 79u8, 23u8, 16u8, 174u8, 243u8, 252u8,
                            42u8, 65u8, 120u8, 229u8, 38u8, 210u8, 255u8, 22u8, 40u8, 109u8, 223u8,
                        ],
                    )
                }
                #[doc = " Total extrinsics count for the current block."]
                pub fn extrinsic_count(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "ExtrinsicCount",
                        vec![],
                        [
                            223u8, 60u8, 201u8, 120u8, 36u8, 44u8, 180u8, 210u8, 242u8, 53u8,
                            222u8, 154u8, 123u8, 176u8, 249u8, 8u8, 225u8, 28u8, 232u8, 4u8, 136u8,
                            41u8, 151u8, 82u8, 189u8, 149u8, 49u8, 166u8, 139u8, 9u8, 163u8, 231u8,
                        ],
                    )
                }
                #[doc = " The current weight for the block."]
                pub fn block_weight(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::frame_support::dispatch::PerDispatchClass<
                            runtime_types::sp_weights::weight_v2::Weight,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "BlockWeight",
                        vec![],
                        [
                            120u8, 67u8, 71u8, 163u8, 36u8, 202u8, 52u8, 106u8, 143u8, 155u8,
                            144u8, 87u8, 142u8, 241u8, 232u8, 183u8, 56u8, 235u8, 27u8, 237u8,
                            20u8, 202u8, 33u8, 85u8, 189u8, 0u8, 28u8, 52u8, 198u8, 40u8, 219u8,
                            54u8,
                        ],
                    )
                }
                #[doc = " Total length (in bytes) for all extrinsics put together, for the current block."]
                pub fn all_extrinsics_len(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "AllExtrinsicsLen",
                        vec![],
                        [
                            202u8, 145u8, 209u8, 225u8, 40u8, 220u8, 174u8, 74u8, 93u8, 164u8,
                            254u8, 248u8, 254u8, 192u8, 32u8, 117u8, 96u8, 149u8, 53u8, 145u8,
                            219u8, 64u8, 234u8, 18u8, 217u8, 200u8, 203u8, 141u8, 145u8, 28u8,
                            134u8, 60u8,
                        ],
                    )
                }
                #[doc = " Map of block numbers to block hashes."]
                pub fn block_hash(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::subxt::ext::sp_core::H256>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "BlockHash",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            50u8, 112u8, 176u8, 239u8, 175u8, 18u8, 205u8, 20u8, 241u8, 195u8,
                            21u8, 228u8, 186u8, 57u8, 200u8, 25u8, 38u8, 44u8, 106u8, 20u8, 168u8,
                            80u8, 76u8, 235u8, 12u8, 51u8, 137u8, 149u8, 200u8, 4u8, 220u8, 237u8,
                        ],
                    )
                }
                #[doc = " Map of block numbers to block hashes."]
                pub fn block_hash_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::subxt::ext::sp_core::H256>,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "BlockHash",
                        Vec::new(),
                        [
                            50u8, 112u8, 176u8, 239u8, 175u8, 18u8, 205u8, 20u8, 241u8, 195u8,
                            21u8, 228u8, 186u8, 57u8, 200u8, 25u8, 38u8, 44u8, 106u8, 20u8, 168u8,
                            80u8, 76u8, 235u8, 12u8, 51u8, 137u8, 149u8, 200u8, 4u8, 220u8, 237u8,
                        ],
                    )
                }
                #[doc = " Extrinsics data for the current block (maps an extrinsic's index to its data)."]
                pub fn extrinsic_data(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::std::vec::Vec<::core::primitive::u8>>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "ExtrinsicData",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            210u8, 224u8, 211u8, 186u8, 118u8, 210u8, 185u8, 194u8, 238u8, 211u8,
                            254u8, 73u8, 67u8, 184u8, 31u8, 229u8, 168u8, 125u8, 98u8, 23u8, 241u8,
                            59u8, 49u8, 86u8, 126u8, 9u8, 114u8, 163u8, 160u8, 62u8, 50u8, 67u8,
                        ],
                    )
                }
                #[doc = " Extrinsics data for the current block (maps an extrinsic's index to its data)."]
                pub fn extrinsic_data_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::std::vec::Vec<::core::primitive::u8>>,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "ExtrinsicData",
                        Vec::new(),
                        [
                            210u8, 224u8, 211u8, 186u8, 118u8, 210u8, 185u8, 194u8, 238u8, 211u8,
                            254u8, 73u8, 67u8, 184u8, 31u8, 229u8, 168u8, 125u8, 98u8, 23u8, 241u8,
                            59u8, 49u8, 86u8, 126u8, 9u8, 114u8, 163u8, 160u8, 62u8, 50u8, 67u8,
                        ],
                    )
                }
                #[doc = " The current block number being processed. Set by `execute_block`."]
                pub fn number(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "Number",
                        vec![],
                        [
                            228u8, 96u8, 102u8, 190u8, 252u8, 130u8, 239u8, 172u8, 126u8, 235u8,
                            246u8, 139u8, 208u8, 15u8, 88u8, 245u8, 141u8, 232u8, 43u8, 204u8,
                            36u8, 87u8, 211u8, 141u8, 187u8, 68u8, 236u8, 70u8, 193u8, 235u8,
                            164u8, 191u8,
                        ],
                    )
                }
                #[doc = " Hash of the previous block."]
                pub fn parent_hash(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::subxt::ext::sp_core::H256>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "ParentHash",
                        vec![],
                        [
                            232u8, 206u8, 177u8, 119u8, 38u8, 57u8, 233u8, 50u8, 225u8, 49u8,
                            169u8, 176u8, 210u8, 51u8, 231u8, 176u8, 234u8, 186u8, 188u8, 112u8,
                            15u8, 152u8, 195u8, 232u8, 201u8, 97u8, 208u8, 249u8, 9u8, 163u8, 69u8,
                            36u8,
                        ],
                    )
                }
                #[doc = " Digest of the current block, also part of the block header."]
                pub fn digest(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_runtime::generic::digest::Digest,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "Digest",
                        vec![],
                        [
                            83u8, 141u8, 200u8, 132u8, 182u8, 55u8, 197u8, 122u8, 13u8, 159u8,
                            31u8, 42u8, 60u8, 191u8, 89u8, 221u8, 242u8, 47u8, 199u8, 213u8, 48u8,
                            216u8, 131u8, 168u8, 245u8, 82u8, 56u8, 190u8, 62u8, 69u8, 96u8, 37u8,
                        ],
                    )
                }
                #[doc = " Events deposited for the current block."]
                #[doc = ""]
                #[doc = " NOTE: The item is unbound and should therefore never be read on chain."]
                #[doc = " It could otherwise inflate the PoV size of a block."]
                #[doc = ""]
                #[doc = " Events have a large in-memory size. Box the events to not go out-of-memory"]
                #[doc = " just in case someone still reads them from within the runtime."]
                pub fn events(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        ::std::vec::Vec<
                            runtime_types::frame_system::EventRecord<
                                runtime_types::aleph_runtime::RuntimeEvent,
                                ::subxt::ext::sp_core::H256,
                            >,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "Events",
                        vec![],
                        [
                            80u8, 163u8, 77u8, 2u8, 114u8, 163u8, 90u8, 227u8, 120u8, 148u8, 222u8,
                            239u8, 255u8, 114u8, 127u8, 214u8, 47u8, 240u8, 5u8, 196u8, 47u8,
                            195u8, 53u8, 197u8, 200u8, 169u8, 89u8, 246u8, 155u8, 60u8, 157u8, 3u8,
                        ],
                    )
                }
                #[doc = " The number of events in the `Events<T>` list."]
                pub fn event_count(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "EventCount",
                        vec![],
                        [
                            236u8, 93u8, 90u8, 177u8, 250u8, 211u8, 138u8, 187u8, 26u8, 208u8,
                            203u8, 113u8, 221u8, 233u8, 227u8, 9u8, 249u8, 25u8, 202u8, 185u8,
                            161u8, 144u8, 167u8, 104u8, 127u8, 187u8, 38u8, 18u8, 52u8, 61u8, 66u8,
                            112u8,
                        ],
                    )
                }
                #[doc = " Mapping between a topic (represented by T::Hash) and a vector of indexes"]
                #[doc = " of events in the `<Events<T>>` list."]
                #[doc = ""]
                #[doc = " All topic vectors have deterministic storage locations depending on the topic. This"]
                #[doc = " allows light-clients to leverage the changes trie storage tracking mechanism and"]
                #[doc = " in case of changes fetch the list of events of interest."]
                #[doc = ""]
                #[doc = " The value has the type `(T::BlockNumber, EventIndex)` because if we used only just"]
                #[doc = " the `EventIndex` then in case if the topic has the same contents on the next block"]
                #[doc = " no notification will be triggered thus the event might be lost."]
                pub fn event_topics(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::H256>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        ::std::vec::Vec<(::core::primitive::u32, ::core::primitive::u32)>,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "EventTopics",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Blake2_128Concat,
                        )],
                        [
                            205u8, 90u8, 142u8, 190u8, 176u8, 37u8, 94u8, 82u8, 98u8, 1u8, 129u8,
                            63u8, 246u8, 101u8, 130u8, 58u8, 216u8, 16u8, 139u8, 196u8, 154u8,
                            111u8, 110u8, 178u8, 24u8, 44u8, 183u8, 176u8, 232u8, 82u8, 223u8,
                            38u8,
                        ],
                    )
                }
                #[doc = " Mapping between a topic (represented by T::Hash) and a vector of indexes"]
                #[doc = " of events in the `<Events<T>>` list."]
                #[doc = ""]
                #[doc = " All topic vectors have deterministic storage locations depending on the topic. This"]
                #[doc = " allows light-clients to leverage the changes trie storage tracking mechanism and"]
                #[doc = " in case of changes fetch the list of events of interest."]
                #[doc = ""]
                #[doc = " The value has the type `(T::BlockNumber, EventIndex)` because if we used only just"]
                #[doc = " the `EventIndex` then in case if the topic has the same contents on the next block"]
                #[doc = " no notification will be triggered thus the event might be lost."]
                pub fn event_topics_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        ::std::vec::Vec<(::core::primitive::u32, ::core::primitive::u32)>,
                    >,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "EventTopics",
                        Vec::new(),
                        [
                            205u8, 90u8, 142u8, 190u8, 176u8, 37u8, 94u8, 82u8, 98u8, 1u8, 129u8,
                            63u8, 246u8, 101u8, 130u8, 58u8, 216u8, 16u8, 139u8, 196u8, 154u8,
                            111u8, 110u8, 178u8, 24u8, 44u8, 183u8, 176u8, 232u8, 82u8, 223u8,
                            38u8,
                        ],
                    )
                }
                #[doc = " Stores the `spec_version` and `spec_name` of when the last runtime upgrade happened."]
                pub fn last_runtime_upgrade(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::frame_system::LastRuntimeUpgradeInfo,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "LastRuntimeUpgrade",
                        vec![],
                        [
                            52u8, 37u8, 117u8, 111u8, 57u8, 130u8, 196u8, 14u8, 99u8, 77u8, 91u8,
                            126u8, 178u8, 249u8, 78u8, 34u8, 9u8, 194u8, 92u8, 105u8, 113u8, 81u8,
                            185u8, 127u8, 245u8, 184u8, 60u8, 29u8, 234u8, 182u8, 96u8, 196u8,
                        ],
                    )
                }
                #[doc = " True if we have upgraded so that `type RefCount` is `u32`. False (default) if not."]
                pub fn upgraded_to_u32_ref_count(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::bool>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "UpgradedToU32RefCount",
                        vec![],
                        [
                            171u8, 88u8, 244u8, 92u8, 122u8, 67u8, 27u8, 18u8, 59u8, 175u8, 175u8,
                            178u8, 20u8, 150u8, 213u8, 59u8, 222u8, 141u8, 32u8, 107u8, 3u8, 114u8,
                            83u8, 250u8, 180u8, 233u8, 152u8, 54u8, 187u8, 99u8, 131u8, 204u8,
                        ],
                    )
                }
                #[doc = " True if we have upgraded so that AccountInfo contains three types of `RefCount`. False"]
                #[doc = " (default) if not."]
                pub fn upgraded_to_triple_ref_count(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::bool>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "UpgradedToTripleRefCount",
                        vec![],
                        [
                            90u8, 33u8, 56u8, 86u8, 90u8, 101u8, 89u8, 133u8, 203u8, 56u8, 201u8,
                            210u8, 244u8, 232u8, 150u8, 18u8, 51u8, 105u8, 14u8, 230u8, 103u8,
                            155u8, 246u8, 99u8, 53u8, 207u8, 225u8, 128u8, 186u8, 76u8, 40u8,
                            185u8,
                        ],
                    )
                }
                #[doc = " The execution phase of the block."]
                pub fn execution_phase(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::frame_system::Phase>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "System",
                        "ExecutionPhase",
                        vec![],
                        [
                            230u8, 183u8, 221u8, 135u8, 226u8, 223u8, 55u8, 104u8, 138u8, 224u8,
                            103u8, 156u8, 222u8, 99u8, 203u8, 199u8, 164u8, 168u8, 193u8, 133u8,
                            201u8, 155u8, 63u8, 95u8, 17u8, 206u8, 165u8, 123u8, 161u8, 33u8,
                            172u8, 93u8,
                        ],
                    )
                }
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                #[doc = " Block & extrinsics weights: base values and limits."]
                pub fn block_weights(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::frame_system::limits::BlockWeights,
                    >,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "System",
                        "BlockWeights",
                        [
                            118u8, 253u8, 239u8, 217u8, 145u8, 115u8, 85u8, 86u8, 172u8, 248u8,
                            139u8, 32u8, 158u8, 126u8, 172u8, 188u8, 197u8, 105u8, 145u8, 235u8,
                            171u8, 50u8, 31u8, 225u8, 167u8, 187u8, 241u8, 87u8, 6u8, 17u8, 234u8,
                            185u8,
                        ],
                    )
                }
                #[doc = " The maximum length of a block (in bytes)."]
                pub fn block_length(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::frame_system::limits::BlockLength,
                    >,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "System",
                        "BlockLength",
                        [
                            116u8, 184u8, 225u8, 228u8, 207u8, 203u8, 4u8, 220u8, 234u8, 198u8,
                            150u8, 108u8, 205u8, 87u8, 194u8, 131u8, 229u8, 51u8, 140u8, 4u8, 47u8,
                            12u8, 200u8, 144u8, 153u8, 62u8, 51u8, 39u8, 138u8, 205u8, 203u8,
                            236u8,
                        ],
                    )
                }
                #[doc = " Maximum number of block number to block hash mappings to keep (oldest pruned first)."]
                pub fn block_hash_count(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "System",
                        "BlockHashCount",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                #[doc = " The weight of runtime database operations the runtime can invoke."]
                pub fn db_weight(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::sp_weights::RuntimeDbWeight>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "System",
                        "DbWeight",
                        [
                            124u8, 162u8, 190u8, 149u8, 49u8, 177u8, 162u8, 231u8, 62u8, 167u8,
                            199u8, 181u8, 43u8, 232u8, 185u8, 116u8, 195u8, 51u8, 233u8, 223u8,
                            20u8, 129u8, 246u8, 13u8, 65u8, 180u8, 64u8, 9u8, 157u8, 59u8, 245u8,
                            118u8,
                        ],
                    )
                }
                #[doc = " Get the chain's current version."]
                pub fn version(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::sp_version::RuntimeVersion>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "System",
                        "Version",
                        [
                            93u8, 98u8, 57u8, 243u8, 229u8, 8u8, 234u8, 231u8, 72u8, 230u8, 139u8,
                            47u8, 63u8, 181u8, 17u8, 2u8, 220u8, 231u8, 104u8, 237u8, 185u8, 143u8,
                            165u8, 253u8, 188u8, 76u8, 147u8, 12u8, 170u8, 26u8, 74u8, 200u8,
                        ],
                    )
                }
                #[doc = " The designated SS58 prefix of this chain."]
                #[doc = ""]
                #[doc = " This replaces the \"ss58Format\" property declared in the chain spec. Reason is"]
                #[doc = " that the runtime should know about the prefix in order to make use of it as"]
                #[doc = " an identifier of the chain."]
                pub fn ss58_prefix(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u16>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "System",
                        "SS58Prefix",
                        [
                            116u8, 33u8, 2u8, 170u8, 181u8, 147u8, 171u8, 169u8, 167u8, 227u8,
                            41u8, 144u8, 11u8, 236u8, 82u8, 100u8, 74u8, 60u8, 184u8, 72u8, 169u8,
                            90u8, 208u8, 135u8, 15u8, 117u8, 10u8, 123u8, 128u8, 193u8, 29u8, 70u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod randomness_collective_flip {
        use super::{root_mod, runtime_types};
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                #[doc = " Series of block headers from the last 81 blocks that acts as random seed material. This"]
                #[doc = " is arranged as a ring buffer with `block_number % 81` being the index into the `Vec` of"]
                #[doc = " the oldest hash."]
                pub fn random_material(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            ::subxt::ext::sp_core::H256,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "RandomnessCollectiveFlip",
                        "RandomMaterial",
                        vec![],
                        [
                            152u8, 126u8, 73u8, 88u8, 54u8, 147u8, 6u8, 19u8, 214u8, 40u8, 159u8,
                            30u8, 236u8, 61u8, 240u8, 65u8, 178u8, 94u8, 146u8, 152u8, 135u8,
                            252u8, 160u8, 86u8, 123u8, 114u8, 251u8, 140u8, 98u8, 143u8, 217u8,
                            242u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod scheduler {
        use super::{root_mod, runtime_types};
        #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Schedule {
                pub when: ::core::primitive::u32,
                pub maybe_periodic:
                    ::core::option::Option<(::core::primitive::u32, ::core::primitive::u32)>,
                pub priority: ::core::primitive::u8,
                pub call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Cancel {
                pub when: ::core::primitive::u32,
                pub index: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ScheduleNamed {
                pub id: [::core::primitive::u8; 32usize],
                pub when: ::core::primitive::u32,
                pub maybe_periodic:
                    ::core::option::Option<(::core::primitive::u32, ::core::primitive::u32)>,
                pub priority: ::core::primitive::u8,
                pub call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct CancelNamed {
                pub id: [::core::primitive::u8; 32usize],
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ScheduleAfter {
                pub after: ::core::primitive::u32,
                pub maybe_periodic:
                    ::core::option::Option<(::core::primitive::u32, ::core::primitive::u32)>,
                pub priority: ::core::primitive::u8,
                pub call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ScheduleNamedAfter {
                pub id: [::core::primitive::u8; 32usize],
                pub after: ::core::primitive::u32,
                pub maybe_periodic:
                    ::core::option::Option<(::core::primitive::u32, ::core::primitive::u32)>,
                pub priority: ::core::primitive::u8,
                pub call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                #[doc = "Anonymously schedule a task."]
                pub fn schedule(
                    &self,
                    when: ::core::primitive::u32,
                    maybe_periodic: ::core::option::Option<(
                        ::core::primitive::u32,
                        ::core::primitive::u32,
                    )>,
                    priority: ::core::primitive::u8,
                    call: runtime_types::aleph_runtime::RuntimeCall,
                ) -> ::subxt::tx::StaticTxPayload<Schedule> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Scheduler",
                        "schedule",
                        Schedule {
                            when,
                            maybe_periodic,
                            priority,
                            call: ::std::boxed::Box::new(call),
                        },
                        [
                            203u8, 163u8, 89u8, 109u8, 45u8, 119u8, 227u8, 27u8, 65u8, 243u8, 46u8,
                            102u8, 187u8, 169u8, 13u8, 66u8, 46u8, 254u8, 113u8, 210u8, 122u8,
                            239u8, 147u8, 107u8, 55u8, 91u8, 1u8, 98u8, 201u8, 123u8, 203u8, 70u8,
                        ],
                    )
                }
                #[doc = "Cancel an anonymously scheduled task."]
                pub fn cancel(
                    &self,
                    when: ::core::primitive::u32,
                    index: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<Cancel> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Scheduler",
                        "cancel",
                        Cancel { when, index },
                        [
                            81u8, 251u8, 234u8, 17u8, 214u8, 75u8, 19u8, 59u8, 19u8, 30u8, 89u8,
                            74u8, 6u8, 216u8, 238u8, 165u8, 7u8, 19u8, 153u8, 253u8, 161u8, 103u8,
                            178u8, 227u8, 152u8, 180u8, 80u8, 156u8, 82u8, 126u8, 132u8, 120u8,
                        ],
                    )
                }
                #[doc = "Schedule a named task."]
                pub fn schedule_named(
                    &self,
                    id: [::core::primitive::u8; 32usize],
                    when: ::core::primitive::u32,
                    maybe_periodic: ::core::option::Option<(
                        ::core::primitive::u32,
                        ::core::primitive::u32,
                    )>,
                    priority: ::core::primitive::u8,
                    call: runtime_types::aleph_runtime::RuntimeCall,
                ) -> ::subxt::tx::StaticTxPayload<ScheduleNamed> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Scheduler",
                        "schedule_named",
                        ScheduleNamed {
                            id,
                            when,
                            maybe_periodic,
                            priority,
                            call: ::std::boxed::Box::new(call),
                        },
                        [
                            18u8, 192u8, 161u8, 114u8, 187u8, 15u8, 78u8, 111u8, 41u8, 148u8, 58u8,
                            78u8, 11u8, 52u8, 58u8, 227u8, 178u8, 168u8, 168u8, 79u8, 23u8, 20u8,
                            191u8, 105u8, 4u8, 119u8, 155u8, 163u8, 240u8, 18u8, 231u8, 88u8,
                        ],
                    )
                }
                #[doc = "Cancel a named scheduled task."]
                pub fn cancel_named(
                    &self,
                    id: [::core::primitive::u8; 32usize],
                ) -> ::subxt::tx::StaticTxPayload<CancelNamed> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Scheduler",
                        "cancel_named",
                        CancelNamed { id },
                        [
                            51u8, 3u8, 140u8, 50u8, 214u8, 211u8, 50u8, 4u8, 19u8, 43u8, 230u8,
                            114u8, 18u8, 108u8, 138u8, 67u8, 99u8, 24u8, 255u8, 11u8, 246u8, 37u8,
                            192u8, 207u8, 90u8, 157u8, 171u8, 93u8, 233u8, 189u8, 64u8, 180u8,
                        ],
                    )
                }
                #[doc = "Anonymously schedule a task after a delay."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "Same as [`schedule`]."]
                #[doc = "# </weight>"]
                pub fn schedule_after(
                    &self,
                    after: ::core::primitive::u32,
                    maybe_periodic: ::core::option::Option<(
                        ::core::primitive::u32,
                        ::core::primitive::u32,
                    )>,
                    priority: ::core::primitive::u8,
                    call: runtime_types::aleph_runtime::RuntimeCall,
                ) -> ::subxt::tx::StaticTxPayload<ScheduleAfter> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Scheduler",
                        "schedule_after",
                        ScheduleAfter {
                            after,
                            maybe_periodic,
                            priority,
                            call: ::std::boxed::Box::new(call),
                        },
                        [
                            19u8, 56u8, 251u8, 84u8, 220u8, 73u8, 53u8, 58u8, 17u8, 107u8, 232u8,
                            203u8, 42u8, 25u8, 57u8, 47u8, 37u8, 190u8, 91u8, 111u8, 92u8, 18u8,
                            22u8, 181u8, 55u8, 73u8, 230u8, 255u8, 87u8, 97u8, 33u8, 19u8,
                        ],
                    )
                }
                #[doc = "Schedule a named task after a delay."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "Same as [`schedule_named`](Self::schedule_named)."]
                #[doc = "# </weight>"]
                pub fn schedule_named_after(
                    &self,
                    id: [::core::primitive::u8; 32usize],
                    after: ::core::primitive::u32,
                    maybe_periodic: ::core::option::Option<(
                        ::core::primitive::u32,
                        ::core::primitive::u32,
                    )>,
                    priority: ::core::primitive::u8,
                    call: runtime_types::aleph_runtime::RuntimeCall,
                ) -> ::subxt::tx::StaticTxPayload<ScheduleNamedAfter> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Scheduler",
                        "schedule_named_after",
                        ScheduleNamedAfter {
                            id,
                            after,
                            maybe_periodic,
                            priority,
                            call: ::std::boxed::Box::new(call),
                        },
                        [
                            11u8, 116u8, 241u8, 215u8, 147u8, 140u8, 121u8, 100u8, 225u8, 1u8,
                            173u8, 155u8, 196u8, 123u8, 9u8, 145u8, 92u8, 180u8, 132u8, 28u8,
                            131u8, 142u8, 146u8, 102u8, 229u8, 151u8, 118u8, 245u8, 20u8, 107u8,
                            170u8, 119u8,
                        ],
                    )
                }
            }
        }
        #[doc = "Events type."]
        pub type Event = runtime_types::pallet_scheduler::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Scheduled some task."]
            pub struct Scheduled {
                pub when: ::core::primitive::u32,
                pub index: ::core::primitive::u32,
            }
            impl ::subxt::events::StaticEvent for Scheduled {
                const PALLET: &'static str = "Scheduler";
                const EVENT: &'static str = "Scheduled";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Canceled some task."]
            pub struct Canceled {
                pub when: ::core::primitive::u32,
                pub index: ::core::primitive::u32,
            }
            impl ::subxt::events::StaticEvent for Canceled {
                const PALLET: &'static str = "Scheduler";
                const EVENT: &'static str = "Canceled";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Dispatched some task."]
            pub struct Dispatched {
                pub task: (::core::primitive::u32, ::core::primitive::u32),
                pub id: ::core::option::Option<[::core::primitive::u8; 32usize]>,
                pub result: ::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
            }
            impl ::subxt::events::StaticEvent for Dispatched {
                const PALLET: &'static str = "Scheduler";
                const EVENT: &'static str = "Dispatched";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "The call for the provided hash was not found so the task has been aborted."]
            pub struct CallUnavailable {
                pub task: (::core::primitive::u32, ::core::primitive::u32),
                pub id: ::core::option::Option<[::core::primitive::u8; 32usize]>,
            }
            impl ::subxt::events::StaticEvent for CallUnavailable {
                const PALLET: &'static str = "Scheduler";
                const EVENT: &'static str = "CallUnavailable";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "The given task was unable to be renewed since the agenda is full at that block."]
            pub struct PeriodicFailed {
                pub task: (::core::primitive::u32, ::core::primitive::u32),
                pub id: ::core::option::Option<[::core::primitive::u8; 32usize]>,
            }
            impl ::subxt::events::StaticEvent for PeriodicFailed {
                const PALLET: &'static str = "Scheduler";
                const EVENT: &'static str = "PeriodicFailed";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "The given task can never be executed since it is overweight."]
            pub struct PermanentlyOverweight {
                pub task: (::core::primitive::u32, ::core::primitive::u32),
                pub id: ::core::option::Option<[::core::primitive::u8; 32usize]>,
            }
            impl ::subxt::events::StaticEvent for PermanentlyOverweight {
                const PALLET: &'static str = "Scheduler";
                const EVENT: &'static str = "PermanentlyOverweight";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                pub fn incomplete_since(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Scheduler",
                        "IncompleteSince",
                        vec![],
                        [
                            149u8, 66u8, 239u8, 67u8, 235u8, 219u8, 101u8, 182u8, 145u8, 56u8,
                            252u8, 150u8, 253u8, 221u8, 125u8, 57u8, 38u8, 152u8, 153u8, 31u8,
                            92u8, 238u8, 66u8, 246u8, 104u8, 163u8, 94u8, 73u8, 222u8, 168u8,
                            193u8, 227u8,
                        ],
                    )
                }
                #[doc = " Items to be executed, indexed by the block number that they should be executed on."]
                pub fn agenda(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            ::core::option::Option<
                                runtime_types::pallet_scheduler::Scheduled<
                                    [::core::primitive::u8; 32usize],
                                    runtime_types::frame_support::traits::preimages::Bounded<
                                        runtime_types::aleph_runtime::RuntimeCall,
                                    >,
                                    ::core::primitive::u32,
                                    runtime_types::aleph_runtime::OriginCaller,
                                    ::subxt::ext::sp_core::crypto::AccountId32,
                                >,
                            >,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Scheduler",
                        "Agenda",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            48u8, 65u8, 163u8, 111u8, 82u8, 33u8, 246u8, 16u8, 83u8, 40u8, 138u8,
                            172u8, 239u8, 30u8, 247u8, 235u8, 100u8, 240u8, 93u8, 50u8, 102u8,
                            203u8, 118u8, 32u8, 174u8, 21u8, 223u8, 91u8, 10u8, 31u8, 75u8, 97u8,
                        ],
                    )
                }
                #[doc = " Items to be executed, indexed by the block number that they should be executed on."]
                pub fn agenda_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            ::core::option::Option<
                                runtime_types::pallet_scheduler::Scheduled<
                                    [::core::primitive::u8; 32usize],
                                    runtime_types::frame_support::traits::preimages::Bounded<
                                        runtime_types::aleph_runtime::RuntimeCall,
                                    >,
                                    ::core::primitive::u32,
                                    runtime_types::aleph_runtime::OriginCaller,
                                    ::subxt::ext::sp_core::crypto::AccountId32,
                                >,
                            >,
                        >,
                    >,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Scheduler",
                        "Agenda",
                        Vec::new(),
                        [
                            48u8, 65u8, 163u8, 111u8, 82u8, 33u8, 246u8, 16u8, 83u8, 40u8, 138u8,
                            172u8, 239u8, 30u8, 247u8, 235u8, 100u8, 240u8, 93u8, 50u8, 102u8,
                            203u8, 118u8, 32u8, 174u8, 21u8, 223u8, 91u8, 10u8, 31u8, 75u8, 97u8,
                        ],
                    )
                }
                #[doc = " Lookup from a name to the block number and index of the task."]
                #[doc = ""]
                #[doc = " For v3 -> v4 the previously unbounded identities are Blake2-256 hashed to form the v4"]
                #[doc = " identities."]
                pub fn lookup(
                    &self,
                    _0: impl ::std::borrow::Borrow<[::core::primitive::u8; 32usize]>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<(
                        ::core::primitive::u32,
                        ::core::primitive::u32,
                    )>,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Scheduler",
                        "Lookup",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            82u8, 20u8, 178u8, 101u8, 108u8, 198u8, 71u8, 99u8, 16u8, 175u8, 15u8,
                            187u8, 229u8, 243u8, 140u8, 200u8, 99u8, 77u8, 248u8, 178u8, 45u8,
                            121u8, 193u8, 67u8, 165u8, 43u8, 234u8, 211u8, 158u8, 250u8, 103u8,
                            243u8,
                        ],
                    )
                }
                #[doc = " Lookup from a name to the block number and index of the task."]
                #[doc = ""]
                #[doc = " For v3 -> v4 the previously unbounded identities are Blake2-256 hashed to form the v4"]
                #[doc = " identities."]
                pub fn lookup_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<(
                        ::core::primitive::u32,
                        ::core::primitive::u32,
                    )>,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Scheduler",
                        "Lookup",
                        Vec::new(),
                        [
                            82u8, 20u8, 178u8, 101u8, 108u8, 198u8, 71u8, 99u8, 16u8, 175u8, 15u8,
                            187u8, 229u8, 243u8, 140u8, 200u8, 99u8, 77u8, 248u8, 178u8, 45u8,
                            121u8, 193u8, 67u8, 165u8, 43u8, 234u8, 211u8, 158u8, 250u8, 103u8,
                            243u8,
                        ],
                    )
                }
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                #[doc = " The maximum weight that may be scheduled per block for any dispatchables."]
                pub fn maximum_weight(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_weights::weight_v2::Weight,
                    >,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Scheduler",
                        "MaximumWeight",
                        [
                            206u8, 61u8, 253u8, 247u8, 163u8, 40u8, 161u8, 52u8, 134u8, 140u8,
                            206u8, 83u8, 44u8, 166u8, 226u8, 115u8, 181u8, 14u8, 227u8, 130u8,
                            210u8, 32u8, 85u8, 29u8, 230u8, 97u8, 130u8, 165u8, 147u8, 134u8,
                            106u8, 76u8,
                        ],
                    )
                }
                #[doc = " The maximum number of scheduled calls in the queue for a single block."]
                pub fn max_scheduled_per_block(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Scheduler",
                        "MaxScheduledPerBlock",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod aura {
        use super::{root_mod, runtime_types};
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                #[doc = " The current authority set."]
                pub fn authorities(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            runtime_types::sp_consensus_aura::sr25519::app_sr25519::Public,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Aura",
                        "Authorities",
                        vec![],
                        [
                            199u8, 89u8, 94u8, 48u8, 249u8, 35u8, 105u8, 90u8, 15u8, 86u8, 218u8,
                            85u8, 22u8, 236u8, 228u8, 36u8, 137u8, 64u8, 236u8, 171u8, 242u8,
                            217u8, 91u8, 240u8, 205u8, 205u8, 226u8, 16u8, 147u8, 235u8, 181u8,
                            41u8,
                        ],
                    )
                }
                #[doc = " The current slot of this block."]
                #[doc = ""]
                #[doc = " This will be set in `on_initialize`."]
                pub fn current_slot(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::sp_consensus_slots::Slot>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Aura",
                        "CurrentSlot",
                        vec![],
                        [
                            139u8, 237u8, 185u8, 137u8, 251u8, 179u8, 69u8, 167u8, 133u8, 168u8,
                            204u8, 64u8, 178u8, 123u8, 92u8, 250u8, 119u8, 190u8, 208u8, 178u8,
                            208u8, 176u8, 124u8, 187u8, 74u8, 165u8, 33u8, 78u8, 161u8, 206u8, 8u8,
                            108u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod timestamp {
        use super::{root_mod, runtime_types};
        #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Set {
                #[codec(compact)]
                pub now: ::core::primitive::u64,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                #[doc = "Set the current time."]
                #[doc = ""]
                #[doc = "This call should be invoked exactly once per block. It will panic at the finalization"]
                #[doc = "phase, if this call hasn't been invoked by that time."]
                #[doc = ""]
                #[doc = "The timestamp should be greater than the previous one by the amount specified by"]
                #[doc = "`MinimumPeriod`."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be `Inherent`."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(1)` (Note that implementations of `OnTimestampSet` must also be `O(1)`)"]
                #[doc = "- 1 storage read and 1 storage mutation (codec `O(1)`). (because of `DidUpdate::take` in"]
                #[doc = "  `on_finalize`)"]
                #[doc = "- 1 event handler `on_timestamp_set`. Must be `O(1)`."]
                #[doc = "# </weight>"]
                pub fn set(
                    &self,
                    now: ::core::primitive::u64,
                ) -> ::subxt::tx::StaticTxPayload<Set> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Timestamp",
                        "set",
                        Set { now },
                        [
                            6u8, 97u8, 172u8, 236u8, 118u8, 238u8, 228u8, 114u8, 15u8, 115u8,
                            102u8, 85u8, 66u8, 151u8, 16u8, 33u8, 187u8, 17u8, 166u8, 88u8, 127u8,
                            214u8, 182u8, 51u8, 168u8, 88u8, 43u8, 101u8, 185u8, 8u8, 1u8, 28u8,
                        ],
                    )
                }
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                #[doc = " Current time for the current block."]
                pub fn now(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u64>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Timestamp",
                        "Now",
                        vec![],
                        [
                            148u8, 53u8, 50u8, 54u8, 13u8, 161u8, 57u8, 150u8, 16u8, 83u8, 144u8,
                            221u8, 59u8, 75u8, 158u8, 130u8, 39u8, 123u8, 106u8, 134u8, 202u8,
                            185u8, 83u8, 85u8, 60u8, 41u8, 120u8, 96u8, 210u8, 34u8, 2u8, 250u8,
                        ],
                    )
                }
                #[doc = " Did the timestamp get updated in this block?"]
                pub fn did_update(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::bool>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Timestamp",
                        "DidUpdate",
                        vec![],
                        [
                            70u8, 13u8, 92u8, 186u8, 80u8, 151u8, 167u8, 90u8, 158u8, 232u8, 175u8,
                            13u8, 103u8, 135u8, 2u8, 78u8, 16u8, 6u8, 39u8, 158u8, 167u8, 85u8,
                            27u8, 47u8, 122u8, 73u8, 127u8, 26u8, 35u8, 168u8, 72u8, 204u8,
                        ],
                    )
                }
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                #[doc = " The minimum period between blocks. Beware that this is different to the *expected*"]
                #[doc = " period that the block production apparatus provides. Your chosen consensus system will"]
                #[doc = " generally work with this to determine a sensible block time. e.g. For Aura, it will be"]
                #[doc = " double this period on default settings."]
                pub fn minimum_period(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u64>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Timestamp",
                        "MinimumPeriod",
                        [
                            128u8, 214u8, 205u8, 242u8, 181u8, 142u8, 124u8, 231u8, 190u8, 146u8,
                            59u8, 226u8, 157u8, 101u8, 103u8, 117u8, 249u8, 65u8, 18u8, 191u8,
                            103u8, 119u8, 53u8, 85u8, 81u8, 96u8, 220u8, 42u8, 184u8, 239u8, 42u8,
                            246u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod balances {
        use super::{root_mod, runtime_types};
        #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Transfer {
                pub dest: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                #[codec(compact)]
                pub value: ::core::primitive::u128,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetBalance {
                pub who: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                #[codec(compact)]
                pub new_free: ::core::primitive::u128,
                #[codec(compact)]
                pub new_reserved: ::core::primitive::u128,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ForceTransfer {
                pub source: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                pub dest: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                #[codec(compact)]
                pub value: ::core::primitive::u128,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct TransferKeepAlive {
                pub dest: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                #[codec(compact)]
                pub value: ::core::primitive::u128,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct TransferAll {
                pub dest: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                pub keep_alive: ::core::primitive::bool,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ForceUnreserve {
                pub who: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                pub amount: ::core::primitive::u128,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                #[doc = "Transfer some liquid free balance to another account."]
                #[doc = ""]
                #[doc = "`transfer` will set the `FreeBalance` of the sender and receiver."]
                #[doc = "If the sender's account is below the existential deposit as a result"]
                #[doc = "of the transfer, the account will be reaped."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be `Signed` by the transactor."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- Dependent on arguments but not critical, given proper implementations for input config"]
                #[doc = "  types. See related functions below."]
                #[doc = "- It contains a limited number of reads and writes internally and no complex"]
                #[doc = "  computation."]
                #[doc = ""]
                #[doc = "Related functions:"]
                #[doc = ""]
                #[doc = "  - `ensure_can_withdraw` is always called internally but has a bounded complexity."]
                #[doc = "  - Transferring balances to accounts that did not exist before will cause"]
                #[doc = "    `T::OnNewAccount::on_new_account` to be called."]
                #[doc = "  - Removing enough funds from an account will trigger `T::DustRemoval::on_unbalanced`."]
                #[doc = "  - `transfer_keep_alive` works the same way as `transfer`, but has an additional check"]
                #[doc = "    that the transfer will not kill the origin account."]
                #[doc = "---------------------------------"]
                #[doc = "- Origin account is already in memory, so no DB operations for them."]
                #[doc = "# </weight>"]
                pub fn transfer(
                    &self,
                    dest: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    value: ::core::primitive::u128,
                ) -> ::subxt::tx::StaticTxPayload<Transfer> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Balances",
                        "transfer",
                        Transfer { dest, value },
                        [
                            111u8, 222u8, 32u8, 56u8, 171u8, 77u8, 252u8, 29u8, 194u8, 155u8,
                            200u8, 192u8, 198u8, 81u8, 23u8, 115u8, 236u8, 91u8, 218u8, 114u8,
                            107u8, 141u8, 138u8, 100u8, 237u8, 21u8, 58u8, 172u8, 3u8, 20u8, 216u8,
                            38u8,
                        ],
                    )
                }
                #[doc = "Set the balances of a given account."]
                #[doc = ""]
                #[doc = "This will alter `FreeBalance` and `ReservedBalance` in storage. it will"]
                #[doc = "also alter the total issuance of the system (`TotalIssuance`) appropriately."]
                #[doc = "If the new free or reserved balance is below the existential deposit,"]
                #[doc = "it will reset the account nonce (`frame_system::AccountNonce`)."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call is `root`."]
                pub fn set_balance(
                    &self,
                    who: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    new_free: ::core::primitive::u128,
                    new_reserved: ::core::primitive::u128,
                ) -> ::subxt::tx::StaticTxPayload<SetBalance> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Balances",
                        "set_balance",
                        SetBalance {
                            who,
                            new_free,
                            new_reserved,
                        },
                        [
                            234u8, 215u8, 97u8, 98u8, 243u8, 199u8, 57u8, 76u8, 59u8, 161u8, 118u8,
                            207u8, 34u8, 197u8, 198u8, 61u8, 231u8, 210u8, 169u8, 235u8, 150u8,
                            137u8, 173u8, 49u8, 28u8, 77u8, 84u8, 149u8, 143u8, 210u8, 139u8,
                            193u8,
                        ],
                    )
                }
                #[doc = "Exactly as `transfer`, except the origin must be root and the source account may be"]
                #[doc = "specified."]
                #[doc = "# <weight>"]
                #[doc = "- Same as transfer, but additional read and write because the source account is not"]
                #[doc = "  assumed to be in the overlay."]
                #[doc = "# </weight>"]
                pub fn force_transfer(
                    &self,
                    source: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    dest: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    value: ::core::primitive::u128,
                ) -> ::subxt::tx::StaticTxPayload<ForceTransfer> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Balances",
                        "force_transfer",
                        ForceTransfer {
                            source,
                            dest,
                            value,
                        },
                        [
                            79u8, 174u8, 212u8, 108u8, 184u8, 33u8, 170u8, 29u8, 232u8, 254u8,
                            195u8, 218u8, 221u8, 134u8, 57u8, 99u8, 6u8, 70u8, 181u8, 227u8, 56u8,
                            239u8, 243u8, 158u8, 157u8, 245u8, 36u8, 162u8, 11u8, 237u8, 147u8,
                            15u8,
                        ],
                    )
                }
                #[doc = "Same as the [`transfer`] call, but with a check that the transfer will not kill the"]
                #[doc = "origin account."]
                #[doc = ""]
                #[doc = "99% of the time you want [`transfer`] instead."]
                #[doc = ""]
                #[doc = "[`transfer`]: struct.Pallet.html#method.transfer"]
                pub fn transfer_keep_alive(
                    &self,
                    dest: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    value: ::core::primitive::u128,
                ) -> ::subxt::tx::StaticTxPayload<TransferKeepAlive> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Balances",
                        "transfer_keep_alive",
                        TransferKeepAlive { dest, value },
                        [
                            112u8, 179u8, 75u8, 168u8, 193u8, 221u8, 9u8, 82u8, 190u8, 113u8,
                            253u8, 13u8, 130u8, 134u8, 170u8, 216u8, 136u8, 111u8, 242u8, 220u8,
                            202u8, 112u8, 47u8, 79u8, 73u8, 244u8, 226u8, 59u8, 240u8, 188u8,
                            210u8, 208u8,
                        ],
                    )
                }
                #[doc = "Transfer the entire transferable balance from the caller account."]
                #[doc = ""]
                #[doc = "NOTE: This function only attempts to transfer _transferable_ balances. This means that"]
                #[doc = "any locked, reserved, or existential deposits (when `keep_alive` is `true`), will not be"]
                #[doc = "transferred by this function. To ensure that this function results in a killed account,"]
                #[doc = "you might need to prepare the account by removing any reference counters, storage"]
                #[doc = "deposits, etc..."]
                #[doc = ""]
                #[doc = "The dispatch origin of this call must be Signed."]
                #[doc = ""]
                #[doc = "- `dest`: The recipient of the transfer."]
                #[doc = "- `keep_alive`: A boolean to determine if the `transfer_all` operation should send all"]
                #[doc = "  of the funds the account has, causing the sender account to be killed (false), or"]
                #[doc = "  transfer everything except at least the existential deposit, which will guarantee to"]
                #[doc = "  keep the sender account alive (true). # <weight>"]
                #[doc = "- O(1). Just like transfer, but reading the user's transferable balance first."]
                #[doc = "  #</weight>"]
                pub fn transfer_all(
                    &self,
                    dest: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    keep_alive: ::core::primitive::bool,
                ) -> ::subxt::tx::StaticTxPayload<TransferAll> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Balances",
                        "transfer_all",
                        TransferAll { dest, keep_alive },
                        [
                            46u8, 129u8, 29u8, 177u8, 221u8, 107u8, 245u8, 69u8, 238u8, 126u8,
                            145u8, 26u8, 219u8, 208u8, 14u8, 80u8, 149u8, 1u8, 214u8, 63u8, 67u8,
                            201u8, 144u8, 45u8, 129u8, 145u8, 174u8, 71u8, 238u8, 113u8, 208u8,
                            34u8,
                        ],
                    )
                }
                #[doc = "Unreserve some balance from a user by force."]
                #[doc = ""]
                #[doc = "Can only be called by ROOT."]
                pub fn force_unreserve(
                    &self,
                    who: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    amount: ::core::primitive::u128,
                ) -> ::subxt::tx::StaticTxPayload<ForceUnreserve> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Balances",
                        "force_unreserve",
                        ForceUnreserve { who, amount },
                        [
                            160u8, 146u8, 137u8, 76u8, 157u8, 187u8, 66u8, 148u8, 207u8, 76u8,
                            32u8, 254u8, 82u8, 215u8, 35u8, 161u8, 213u8, 52u8, 32u8, 98u8, 102u8,
                            106u8, 234u8, 123u8, 6u8, 175u8, 184u8, 188u8, 174u8, 106u8, 176u8,
                            78u8,
                        ],
                    )
                }
            }
        }
        #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
        pub type Event = runtime_types::pallet_balances::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "An account was created with some free balance."]
            pub struct Endowed {
                pub account: ::subxt::ext::sp_core::crypto::AccountId32,
                pub free_balance: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Endowed {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "Endowed";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "An account was removed whose balance was non-zero but below ExistentialDeposit,"]
            #[doc = "resulting in an outright loss."]
            pub struct DustLost {
                pub account: ::subxt::ext::sp_core::crypto::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for DustLost {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "DustLost";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Transfer succeeded."]
            pub struct Transfer {
                pub from: ::subxt::ext::sp_core::crypto::AccountId32,
                pub to: ::subxt::ext::sp_core::crypto::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Transfer {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "Transfer";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A balance was set by root."]
            pub struct BalanceSet {
                pub who: ::subxt::ext::sp_core::crypto::AccountId32,
                pub free: ::core::primitive::u128,
                pub reserved: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for BalanceSet {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "BalanceSet";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Some balance was reserved (moved from free to reserved)."]
            pub struct Reserved {
                pub who: ::subxt::ext::sp_core::crypto::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Reserved {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "Reserved";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Some balance was unreserved (moved from reserved to free)."]
            pub struct Unreserved {
                pub who: ::subxt::ext::sp_core::crypto::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Unreserved {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "Unreserved";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Some balance was moved from the reserve of the first account to the second account."]
            #[doc = "Final argument indicates the destination balance type."]
            pub struct ReserveRepatriated {
                pub from: ::subxt::ext::sp_core::crypto::AccountId32,
                pub to: ::subxt::ext::sp_core::crypto::AccountId32,
                pub amount: ::core::primitive::u128,
                pub destination_status:
                    runtime_types::frame_support::traits::tokens::misc::BalanceStatus,
            }
            impl ::subxt::events::StaticEvent for ReserveRepatriated {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "ReserveRepatriated";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Some amount was deposited (e.g. for transaction fees)."]
            pub struct Deposit {
                pub who: ::subxt::ext::sp_core::crypto::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Deposit {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "Deposit";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Some amount was withdrawn from the account (e.g. for transaction fees)."]
            pub struct Withdraw {
                pub who: ::subxt::ext::sp_core::crypto::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Withdraw {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "Withdraw";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Some amount was removed from the account (e.g. for misbehavior)."]
            pub struct Slashed {
                pub who: ::subxt::ext::sp_core::crypto::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Slashed {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "Slashed";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                #[doc = " The total units issued in the system."]
                pub fn total_issuance(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Balances",
                        "TotalIssuance",
                        vec![],
                        [
                            1u8, 206u8, 252u8, 237u8, 6u8, 30u8, 20u8, 232u8, 164u8, 115u8, 51u8,
                            156u8, 156u8, 206u8, 241u8, 187u8, 44u8, 84u8, 25u8, 164u8, 235u8,
                            20u8, 86u8, 242u8, 124u8, 23u8, 28u8, 140u8, 26u8, 73u8, 231u8, 51u8,
                        ],
                    )
                }
                #[doc = " The total units of outstanding deactivated balance in the system."]
                pub fn inactive_issuance(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Balances",
                        "InactiveIssuance",
                        vec![],
                        [
                            74u8, 203u8, 111u8, 142u8, 225u8, 104u8, 173u8, 51u8, 226u8, 12u8,
                            85u8, 135u8, 41u8, 206u8, 177u8, 238u8, 94u8, 246u8, 184u8, 250u8,
                            140u8, 213u8, 91u8, 118u8, 163u8, 111u8, 211u8, 46u8, 204u8, 160u8,
                            154u8, 21u8,
                        ],
                    )
                }
                #[doc = " The Balances pallet example of storing the balance of an account."]
                #[doc = ""]
                #[doc = " # Example"]
                #[doc = ""]
                #[doc = " ```nocompile"]
                #[doc = "  impl pallet_balances::Config for Runtime {"]
                #[doc = "    type AccountStore = StorageMapShim<Self::Account<Runtime>, frame_system::Provider<Runtime>, AccountId, Self::AccountData<Balance>>"]
                #[doc = "  }"]
                #[doc = " ```"]
                #[doc = ""]
                #[doc = " You can also store the balance of an account in the `System` pallet."]
                #[doc = ""]
                #[doc = " # Example"]
                #[doc = ""]
                #[doc = " ```nocompile"]
                #[doc = "  impl pallet_balances::Config for Runtime {"]
                #[doc = "   type AccountStore = System"]
                #[doc = "  }"]
                #[doc = " ```"]
                #[doc = ""]
                #[doc = " But this comes with tradeoffs, storing account balances in the system pallet stores"]
                #[doc = " `frame_system` data alongside the account data contrary to storing account balances in the"]
                #[doc = " `Balances` pallet, which uses a `StorageMap` to store balances data only."]
                #[doc = " NOTE: This is only used in the case that this pallet is used to store balances."]
                pub fn account(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_balances::AccountData<::core::primitive::u128>,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Balances",
                        "Account",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Blake2_128Concat,
                        )],
                        [
                            246u8, 154u8, 253u8, 71u8, 192u8, 192u8, 192u8, 236u8, 128u8, 80u8,
                            40u8, 252u8, 201u8, 43u8, 3u8, 131u8, 19u8, 49u8, 141u8, 240u8, 172u8,
                            217u8, 215u8, 109u8, 87u8, 135u8, 248u8, 57u8, 98u8, 185u8, 22u8, 4u8,
                        ],
                    )
                }
                #[doc = " The Balances pallet example of storing the balance of an account."]
                #[doc = ""]
                #[doc = " # Example"]
                #[doc = ""]
                #[doc = " ```nocompile"]
                #[doc = "  impl pallet_balances::Config for Runtime {"]
                #[doc = "    type AccountStore = StorageMapShim<Self::Account<Runtime>, frame_system::Provider<Runtime>, AccountId, Self::AccountData<Balance>>"]
                #[doc = "  }"]
                #[doc = " ```"]
                #[doc = ""]
                #[doc = " You can also store the balance of an account in the `System` pallet."]
                #[doc = ""]
                #[doc = " # Example"]
                #[doc = ""]
                #[doc = " ```nocompile"]
                #[doc = "  impl pallet_balances::Config for Runtime {"]
                #[doc = "   type AccountStore = System"]
                #[doc = "  }"]
                #[doc = " ```"]
                #[doc = ""]
                #[doc = " But this comes with tradeoffs, storing account balances in the system pallet stores"]
                #[doc = " `frame_system` data alongside the account data contrary to storing account balances in the"]
                #[doc = " `Balances` pallet, which uses a `StorageMap` to store balances data only."]
                #[doc = " NOTE: This is only used in the case that this pallet is used to store balances."]
                pub fn account_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_balances::AccountData<::core::primitive::u128>,
                    >,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Balances",
                        "Account",
                        Vec::new(),
                        [
                            246u8, 154u8, 253u8, 71u8, 192u8, 192u8, 192u8, 236u8, 128u8, 80u8,
                            40u8, 252u8, 201u8, 43u8, 3u8, 131u8, 19u8, 49u8, 141u8, 240u8, 172u8,
                            217u8, 215u8, 109u8, 87u8, 135u8, 248u8, 57u8, 98u8, 185u8, 22u8, 4u8,
                        ],
                    )
                }
                #[doc = " Any liquidity locks on some account balances."]
                #[doc = " NOTE: Should only be accessed when setting, changing and freeing a lock."]
                pub fn locks(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::weak_bounded_vec::WeakBoundedVec<
                            runtime_types::pallet_balances::BalanceLock<::core::primitive::u128>,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Balances",
                        "Locks",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Blake2_128Concat,
                        )],
                        [
                            216u8, 253u8, 87u8, 73u8, 24u8, 218u8, 35u8, 0u8, 244u8, 134u8, 195u8,
                            58u8, 255u8, 64u8, 153u8, 212u8, 210u8, 232u8, 4u8, 122u8, 90u8, 212u8,
                            136u8, 14u8, 127u8, 232u8, 8u8, 192u8, 40u8, 233u8, 18u8, 250u8,
                        ],
                    )
                }
                #[doc = " Any liquidity locks on some account balances."]
                #[doc = " NOTE: Should only be accessed when setting, changing and freeing a lock."]
                pub fn locks_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::weak_bounded_vec::WeakBoundedVec<
                            runtime_types::pallet_balances::BalanceLock<::core::primitive::u128>,
                        >,
                    >,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Balances",
                        "Locks",
                        Vec::new(),
                        [
                            216u8, 253u8, 87u8, 73u8, 24u8, 218u8, 35u8, 0u8, 244u8, 134u8, 195u8,
                            58u8, 255u8, 64u8, 153u8, 212u8, 210u8, 232u8, 4u8, 122u8, 90u8, 212u8,
                            136u8, 14u8, 127u8, 232u8, 8u8, 192u8, 40u8, 233u8, 18u8, 250u8,
                        ],
                    )
                }
                #[doc = " Named reserves on some account balances."]
                pub fn reserves(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            runtime_types::pallet_balances::ReserveData<
                                [::core::primitive::u8; 8usize],
                                ::core::primitive::u128,
                            >,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Balances",
                        "Reserves",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Blake2_128Concat,
                        )],
                        [
                            17u8, 32u8, 191u8, 46u8, 76u8, 220u8, 101u8, 100u8, 42u8, 250u8, 128u8,
                            167u8, 117u8, 44u8, 85u8, 96u8, 105u8, 216u8, 16u8, 147u8, 74u8, 55u8,
                            183u8, 94u8, 160u8, 177u8, 26u8, 187u8, 71u8, 197u8, 187u8, 163u8,
                        ],
                    )
                }
                #[doc = " Named reserves on some account balances."]
                pub fn reserves_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            runtime_types::pallet_balances::ReserveData<
                                [::core::primitive::u8; 8usize],
                                ::core::primitive::u128,
                            >,
                        >,
                    >,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Balances",
                        "Reserves",
                        Vec::new(),
                        [
                            17u8, 32u8, 191u8, 46u8, 76u8, 220u8, 101u8, 100u8, 42u8, 250u8, 128u8,
                            167u8, 117u8, 44u8, 85u8, 96u8, 105u8, 216u8, 16u8, 147u8, 74u8, 55u8,
                            183u8, 94u8, 160u8, 177u8, 26u8, 187u8, 71u8, 197u8, 187u8, 163u8,
                        ],
                    )
                }
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                #[doc = " The minimum amount required to keep an account open."]
                pub fn existential_deposit(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Balances",
                        "ExistentialDeposit",
                        [
                            84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
                            27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
                            136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
                        ],
                    )
                }
                #[doc = " The maximum number of locks that should exist on an account."]
                #[doc = " Not strictly enforced, but used for weight estimation."]
                pub fn max_locks(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Balances",
                        "MaxLocks",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                #[doc = " The maximum number of named reserves that can exist on an account."]
                pub fn max_reserves(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Balances",
                        "MaxReserves",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod transaction_payment {
        use super::{root_mod, runtime_types};
        #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
        pub type Event = runtime_types::pallet_transaction_payment::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A transaction fee `actual_fee`, of which `tip` was added to the minimum inclusion fee,"]
            #[doc = "has been paid by `who`."]
            pub struct TransactionFeePaid {
                pub who: ::subxt::ext::sp_core::crypto::AccountId32,
                pub actual_fee: ::core::primitive::u128,
                pub tip: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for TransactionFeePaid {
                const PALLET: &'static str = "TransactionPayment";
                const EVENT: &'static str = "TransactionFeePaid";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                pub fn next_fee_multiplier(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_arithmetic::fixed_point::FixedU128,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "TransactionPayment",
                        "NextFeeMultiplier",
                        vec![],
                        [
                            210u8, 0u8, 206u8, 165u8, 183u8, 10u8, 206u8, 52u8, 14u8, 90u8, 218u8,
                            197u8, 189u8, 125u8, 113u8, 216u8, 52u8, 161u8, 45u8, 24u8, 245u8,
                            237u8, 121u8, 41u8, 106u8, 29u8, 45u8, 129u8, 250u8, 203u8, 206u8,
                            180u8,
                        ],
                    )
                }
                pub fn storage_version(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_transaction_payment::Releases,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "TransactionPayment",
                        "StorageVersion",
                        vec![],
                        [
                            219u8, 243u8, 82u8, 176u8, 65u8, 5u8, 132u8, 114u8, 8u8, 82u8, 176u8,
                            200u8, 97u8, 150u8, 177u8, 164u8, 166u8, 11u8, 34u8, 12u8, 12u8, 198u8,
                            58u8, 191u8, 186u8, 221u8, 221u8, 119u8, 181u8, 253u8, 154u8, 228u8,
                        ],
                    )
                }
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                #[doc = " A fee mulitplier for `Operational` extrinsics to compute \"virtual tip\" to boost their"]
                #[doc = " `priority`"]
                #[doc = ""]
                #[doc = " This value is multipled by the `final_fee` to obtain a \"virtual tip\" that is later"]
                #[doc = " added to a tip component in regular `priority` calculations."]
                #[doc = " It means that a `Normal` transaction can front-run a similarly-sized `Operational`"]
                #[doc = " extrinsic (with no tip), by including a tip value greater than the virtual tip."]
                #[doc = ""]
                #[doc = " ```rust,ignore"]
                #[doc = " // For `Normal`"]
                #[doc = " let priority = priority_calc(tip);"]
                #[doc = ""]
                #[doc = " // For `Operational`"]
                #[doc = " let virtual_tip = (inclusion_fee + tip) * OperationalFeeMultiplier;"]
                #[doc = " let priority = priority_calc(tip + virtual_tip);"]
                #[doc = " ```"]
                #[doc = ""]
                #[doc = " Note that since we use `final_fee` the multiplier applies also to the regular `tip`"]
                #[doc = " sent with the transaction. So, not only does the transaction get a priority bump based"]
                #[doc = " on the `inclusion_fee`, but we also amplify the impact of tips applied to `Operational`"]
                #[doc = " transactions."]
                pub fn operational_fee_multiplier(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u8>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "TransactionPayment",
                        "OperationalFeeMultiplier",
                        [
                            141u8, 130u8, 11u8, 35u8, 226u8, 114u8, 92u8, 179u8, 168u8, 110u8,
                            28u8, 91u8, 221u8, 64u8, 4u8, 148u8, 201u8, 193u8, 185u8, 66u8, 226u8,
                            114u8, 97u8, 79u8, 62u8, 212u8, 202u8, 114u8, 237u8, 228u8, 183u8,
                            165u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod authorship {
        use super::{root_mod, runtime_types};
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                #[doc = " Author of current block."]
                pub fn author(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::subxt::ext::sp_core::crypto::AccountId32>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Authorship",
                        "Author",
                        vec![],
                        [
                            149u8, 42u8, 33u8, 147u8, 190u8, 207u8, 174u8, 227u8, 190u8, 110u8,
                            25u8, 131u8, 5u8, 167u8, 237u8, 188u8, 188u8, 33u8, 177u8, 126u8,
                            181u8, 49u8, 126u8, 118u8, 46u8, 128u8, 154u8, 95u8, 15u8, 91u8, 103u8,
                            113u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod staking {
        use super::{root_mod, runtime_types};
        #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Bond {
                pub controller: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                #[codec(compact)]
                pub value: ::core::primitive::u128,
                pub payee: runtime_types::pallet_staking::RewardDestination<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct BondExtra {
                #[codec(compact)]
                pub max_additional: ::core::primitive::u128,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Unbond {
                #[codec(compact)]
                pub value: ::core::primitive::u128,
            }
            #[derive(
                :: subxt :: ext :: codec :: CompactAs,
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct WithdrawUnbonded {
                pub num_slashing_spans: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Validate {
                pub prefs: runtime_types::pallet_staking::ValidatorPrefs,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Nominate {
                pub targets: ::std::vec::Vec<
                    ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Chill;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetPayee {
                pub payee: runtime_types::pallet_staking::RewardDestination<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetController {
                pub controller: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetValidatorCount {
                #[codec(compact)]
                pub new: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct IncreaseValidatorCount {
                #[codec(compact)]
                pub additional: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ScaleValidatorCount {
                pub factor: runtime_types::sp_arithmetic::per_things::Percent,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ForceNoEras;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ForceNewEra;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetInvulnerables {
                pub invulnerables: ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ForceUnstake {
                pub stash: ::subxt::ext::sp_core::crypto::AccountId32,
                pub num_slashing_spans: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ForceNewEraAlways;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct CancelDeferredSlash {
                pub era: ::core::primitive::u32,
                pub slash_indices: ::std::vec::Vec<::core::primitive::u32>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct PayoutStakers {
                pub validator_stash: ::subxt::ext::sp_core::crypto::AccountId32,
                pub era: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Rebond {
                #[codec(compact)]
                pub value: ::core::primitive::u128,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ReapStash {
                pub stash: ::subxt::ext::sp_core::crypto::AccountId32,
                pub num_slashing_spans: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Kick {
                pub who: ::std::vec::Vec<
                    ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetStakingConfigs {
                pub min_nominator_bond: runtime_types::pallet_staking::pallet::pallet::ConfigOp<
                    ::core::primitive::u128,
                >,
                pub min_validator_bond: runtime_types::pallet_staking::pallet::pallet::ConfigOp<
                    ::core::primitive::u128,
                >,
                pub max_nominator_count:
                    runtime_types::pallet_staking::pallet::pallet::ConfigOp<::core::primitive::u32>,
                pub max_validator_count:
                    runtime_types::pallet_staking::pallet::pallet::ConfigOp<::core::primitive::u32>,
                pub chill_threshold: runtime_types::pallet_staking::pallet::pallet::ConfigOp<
                    runtime_types::sp_arithmetic::per_things::Percent,
                >,
                pub min_commission: runtime_types::pallet_staking::pallet::pallet::ConfigOp<
                    runtime_types::sp_arithmetic::per_things::Perbill,
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ChillOther {
                pub controller: ::subxt::ext::sp_core::crypto::AccountId32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ForceApplyMinCommission {
                pub validator_stash: ::subxt::ext::sp_core::crypto::AccountId32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetMinCommission {
                pub new: runtime_types::sp_arithmetic::per_things::Perbill,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                #[doc = "Take the origin account as a stash and lock up `value` of its balance. `controller` will"]
                #[doc = "be the account that controls it."]
                #[doc = ""]
                #[doc = "`value` must be more than the `minimum_balance` specified by `T::Currency`."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ by the stash account."]
                #[doc = ""]
                #[doc = "Emits `Bonded`."]
                #[doc = "# <weight>"]
                #[doc = "- Independent of the arguments. Moderate complexity."]
                #[doc = "- O(1)."]
                #[doc = "- Three extra DB entries."]
                #[doc = ""]
                #[doc = "NOTE: Two of the storage writes (`Self::bonded`, `Self::payee`) are _never_ cleaned"]
                #[doc = "unless the `origin` falls below _existential deposit_ and gets removed as dust."]
                #[doc = "------------------"]
                #[doc = "# </weight>"]
                pub fn bond(
                    &self,
                    controller: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    value: ::core::primitive::u128,
                    payee: runtime_types::pallet_staking::RewardDestination<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                    >,
                ) -> ::subxt::tx::StaticTxPayload<Bond> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "bond",
                        Bond {
                            controller,
                            value,
                            payee,
                        },
                        [
                            215u8, 211u8, 69u8, 215u8, 33u8, 158u8, 62u8, 3u8, 31u8, 216u8, 213u8,
                            188u8, 151u8, 43u8, 165u8, 154u8, 117u8, 163u8, 190u8, 227u8, 116u8,
                            70u8, 155u8, 178u8, 64u8, 174u8, 203u8, 179u8, 214u8, 187u8, 176u8,
                            10u8,
                        ],
                    )
                }
                #[doc = "Add some extra amount that have appeared in the stash `free_balance` into the balance up"]
                #[doc = "for staking."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ by the stash, not the controller."]
                #[doc = ""]
                #[doc = "Use this if there are additional funds in your stash account that you wish to bond."]
                #[doc = "Unlike [`bond`](Self::bond) or [`unbond`](Self::unbond) this function does not impose"]
                #[doc = "any limitation on the amount that can be added."]
                #[doc = ""]
                #[doc = "Emits `Bonded`."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- Independent of the arguments. Insignificant complexity."]
                #[doc = "- O(1)."]
                #[doc = "# </weight>"]
                pub fn bond_extra(
                    &self,
                    max_additional: ::core::primitive::u128,
                ) -> ::subxt::tx::StaticTxPayload<BondExtra> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "bond_extra",
                        BondExtra { max_additional },
                        [
                            60u8, 45u8, 82u8, 223u8, 113u8, 95u8, 0u8, 71u8, 59u8, 108u8, 228u8,
                            9u8, 95u8, 210u8, 113u8, 106u8, 252u8, 15u8, 19u8, 128u8, 11u8, 187u8,
                            4u8, 151u8, 103u8, 143u8, 24u8, 33u8, 149u8, 82u8, 35u8, 192u8,
                        ],
                    )
                }
                #[doc = "Schedule a portion of the stash to be unlocked ready for transfer out after the bond"]
                #[doc = "period ends. If this leaves an amount actively bonded less than"]
                #[doc = "T::Currency::minimum_balance(), then it is increased to the full amount."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ by the controller, not the stash."]
                #[doc = ""]
                #[doc = "Once the unlock period is done, you can call `withdraw_unbonded` to actually move"]
                #[doc = "the funds out of management ready for transfer."]
                #[doc = ""]
                #[doc = "No more than a limited number of unlocking chunks (see `MaxUnlockingChunks`)"]
                #[doc = "can co-exists at the same time. If there are no unlocking chunks slots available"]
                #[doc = "[`Call::withdraw_unbonded`] is called to remove some of the chunks (if possible)."]
                #[doc = ""]
                #[doc = "If a user encounters the `InsufficientBond` error when calling this extrinsic,"]
                #[doc = "they should call `chill` first in order to free up their bonded funds."]
                #[doc = ""]
                #[doc = "Emits `Unbonded`."]
                #[doc = ""]
                #[doc = "See also [`Call::withdraw_unbonded`]."]
                pub fn unbond(
                    &self,
                    value: ::core::primitive::u128,
                ) -> ::subxt::tx::StaticTxPayload<Unbond> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "unbond",
                        Unbond { value },
                        [
                            85u8, 62u8, 34u8, 127u8, 60u8, 241u8, 134u8, 60u8, 125u8, 91u8, 31u8,
                            193u8, 50u8, 230u8, 237u8, 42u8, 114u8, 230u8, 240u8, 146u8, 14u8,
                            109u8, 185u8, 151u8, 148u8, 44u8, 147u8, 182u8, 192u8, 253u8, 51u8,
                            87u8,
                        ],
                    )
                }
                #[doc = "Remove any unlocked chunks from the `unlocking` queue from our management."]
                #[doc = ""]
                #[doc = "This essentially frees up that balance to be used by the stash account to do"]
                #[doc = "whatever it wants."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ by the controller."]
                #[doc = ""]
                #[doc = "Emits `Withdrawn`."]
                #[doc = ""]
                #[doc = "See also [`Call::unbond`]."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "Complexity O(S) where S is the number of slashing spans to remove"]
                #[doc = "NOTE: Weight annotation is the kill scenario, we refund otherwise."]
                #[doc = "# </weight>"]
                pub fn withdraw_unbonded(
                    &self,
                    num_slashing_spans: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<WithdrawUnbonded> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "withdraw_unbonded",
                        WithdrawUnbonded { num_slashing_spans },
                        [
                            95u8, 223u8, 122u8, 217u8, 76u8, 208u8, 86u8, 129u8, 31u8, 104u8, 70u8,
                            154u8, 23u8, 250u8, 165u8, 192u8, 149u8, 249u8, 158u8, 159u8, 194u8,
                            224u8, 118u8, 134u8, 204u8, 157u8, 72u8, 136u8, 19u8, 193u8, 183u8,
                            84u8,
                        ],
                    )
                }
                #[doc = "Declare the desire to validate for the origin controller."]
                #[doc = ""]
                #[doc = "Effects will be felt at the beginning of the next era."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ by the controller, not the stash."]
                pub fn validate(
                    &self,
                    prefs: runtime_types::pallet_staking::ValidatorPrefs,
                ) -> ::subxt::tx::StaticTxPayload<Validate> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "validate",
                        Validate { prefs },
                        [
                            191u8, 116u8, 139u8, 35u8, 250u8, 211u8, 86u8, 240u8, 35u8, 9u8, 19u8,
                            44u8, 148u8, 35u8, 91u8, 106u8, 200u8, 172u8, 108u8, 145u8, 194u8,
                            146u8, 61u8, 145u8, 233u8, 168u8, 2u8, 26u8, 145u8, 101u8, 114u8,
                            157u8,
                        ],
                    )
                }
                #[doc = "Declare the desire to nominate `targets` for the origin controller."]
                #[doc = ""]
                #[doc = "Effects will be felt at the beginning of the next era."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ by the controller, not the stash."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- The transaction's complexity is proportional to the size of `targets` (N)"]
                #[doc = "which is capped at CompactAssignments::LIMIT (T::MaxNominations)."]
                #[doc = "- Both the reads and writes follow a similar pattern."]
                #[doc = "# </weight>"]
                pub fn nominate(
                    &self,
                    targets: ::std::vec::Vec<
                        ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                    >,
                ) -> ::subxt::tx::StaticTxPayload<Nominate> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "nominate",
                        Nominate { targets },
                        [
                            112u8, 162u8, 70u8, 26u8, 74u8, 7u8, 188u8, 193u8, 210u8, 247u8, 27u8,
                            189u8, 133u8, 137u8, 33u8, 155u8, 255u8, 171u8, 122u8, 68u8, 175u8,
                            247u8, 139u8, 253u8, 97u8, 187u8, 254u8, 201u8, 66u8, 166u8, 226u8,
                            90u8,
                        ],
                    )
                }
                #[doc = "Declare no desire to either validate or nominate."]
                #[doc = ""]
                #[doc = "Effects will be felt at the beginning of the next era."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ by the controller, not the stash."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- Independent of the arguments. Insignificant complexity."]
                #[doc = "- Contains one read."]
                #[doc = "- Writes are limited to the `origin` account key."]
                #[doc = "# </weight>"]
                pub fn chill(&self) -> ::subxt::tx::StaticTxPayload<Chill> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "chill",
                        Chill {},
                        [
                            94u8, 20u8, 196u8, 31u8, 220u8, 125u8, 115u8, 167u8, 140u8, 3u8, 20u8,
                            132u8, 81u8, 120u8, 215u8, 166u8, 230u8, 56u8, 16u8, 222u8, 31u8,
                            153u8, 120u8, 62u8, 153u8, 67u8, 220u8, 239u8, 11u8, 234u8, 127u8,
                            122u8,
                        ],
                    )
                }
                #[doc = "(Re-)set the payment target for a controller."]
                #[doc = ""]
                #[doc = "Effects will be felt instantly (as soon as this function is completed successfully)."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ by the controller, not the stash."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- Independent of the arguments. Insignificant complexity."]
                #[doc = "- Contains a limited number of reads."]
                #[doc = "- Writes are limited to the `origin` account key."]
                #[doc = "---------"]
                #[doc = "- Weight: O(1)"]
                #[doc = "- DB Weight:"]
                #[doc = "    - Read: Ledger"]
                #[doc = "    - Write: Payee"]
                #[doc = "# </weight>"]
                pub fn set_payee(
                    &self,
                    payee: runtime_types::pallet_staking::RewardDestination<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                    >,
                ) -> ::subxt::tx::StaticTxPayload<SetPayee> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "set_payee",
                        SetPayee { payee },
                        [
                            96u8, 8u8, 254u8, 164u8, 87u8, 46u8, 120u8, 11u8, 197u8, 63u8, 20u8,
                            178u8, 167u8, 236u8, 149u8, 245u8, 14u8, 171u8, 108u8, 195u8, 250u8,
                            133u8, 0u8, 75u8, 192u8, 159u8, 84u8, 220u8, 242u8, 133u8, 60u8, 62u8,
                        ],
                    )
                }
                #[doc = "(Re-)set the controller of a stash."]
                #[doc = ""]
                #[doc = "Effects will be felt instantly (as soon as this function is completed successfully)."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ by the stash, not the controller."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- Independent of the arguments. Insignificant complexity."]
                #[doc = "- Contains a limited number of reads."]
                #[doc = "- Writes are limited to the `origin` account key."]
                #[doc = "----------"]
                #[doc = "Weight: O(1)"]
                #[doc = "DB Weight:"]
                #[doc = "- Read: Bonded, Ledger New Controller, Ledger Old Controller"]
                #[doc = "- Write: Bonded, Ledger New Controller, Ledger Old Controller"]
                #[doc = "# </weight>"]
                pub fn set_controller(
                    &self,
                    controller: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                ) -> ::subxt::tx::StaticTxPayload<SetController> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "set_controller",
                        SetController { controller },
                        [
                            165u8, 250u8, 213u8, 32u8, 179u8, 163u8, 15u8, 35u8, 14u8, 152u8, 56u8,
                            171u8, 43u8, 101u8, 7u8, 167u8, 178u8, 60u8, 89u8, 186u8, 59u8, 28u8,
                            82u8, 159u8, 13u8, 96u8, 168u8, 123u8, 194u8, 212u8, 205u8, 184u8,
                        ],
                    )
                }
                #[doc = "Sets the ideal number of validators."]
                #[doc = ""]
                #[doc = "The dispatch origin must be Root."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "Weight: O(1)"]
                #[doc = "Write: Validator Count"]
                #[doc = "# </weight>"]
                pub fn set_validator_count(
                    &self,
                    new: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<SetValidatorCount> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "set_validator_count",
                        SetValidatorCount { new },
                        [
                            55u8, 232u8, 95u8, 66u8, 228u8, 217u8, 11u8, 27u8, 3u8, 202u8, 199u8,
                            242u8, 70u8, 160u8, 250u8, 187u8, 194u8, 91u8, 15u8, 36u8, 215u8, 36u8,
                            160u8, 108u8, 251u8, 60u8, 240u8, 202u8, 249u8, 235u8, 28u8, 94u8,
                        ],
                    )
                }
                #[doc = "Increments the ideal number of validators upto maximum of"]
                #[doc = "`ElectionProviderBase::MaxWinners`."]
                #[doc = ""]
                #[doc = "The dispatch origin must be Root."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "Same as [`Self::set_validator_count`]."]
                #[doc = "# </weight>"]
                pub fn increase_validator_count(
                    &self,
                    additional: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<IncreaseValidatorCount> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "increase_validator_count",
                        IncreaseValidatorCount { additional },
                        [
                            239u8, 184u8, 155u8, 213u8, 25u8, 22u8, 193u8, 13u8, 102u8, 192u8,
                            82u8, 153u8, 249u8, 192u8, 60u8, 158u8, 8u8, 78u8, 175u8, 219u8, 46u8,
                            51u8, 222u8, 193u8, 193u8, 201u8, 78u8, 90u8, 58u8, 86u8, 196u8, 17u8,
                        ],
                    )
                }
                #[doc = "Scale up the ideal number of validators by a factor upto maximum of"]
                #[doc = "`ElectionProviderBase::MaxWinners`."]
                #[doc = ""]
                #[doc = "The dispatch origin must be Root."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "Same as [`Self::set_validator_count`]."]
                #[doc = "# </weight>"]
                pub fn scale_validator_count(
                    &self,
                    factor: runtime_types::sp_arithmetic::per_things::Percent,
                ) -> ::subxt::tx::StaticTxPayload<ScaleValidatorCount> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "scale_validator_count",
                        ScaleValidatorCount { factor },
                        [
                            198u8, 68u8, 227u8, 94u8, 110u8, 157u8, 209u8, 217u8, 112u8, 37u8,
                            78u8, 142u8, 12u8, 193u8, 219u8, 167u8, 149u8, 112u8, 49u8, 139u8,
                            74u8, 81u8, 172u8, 72u8, 253u8, 224u8, 56u8, 194u8, 185u8, 90u8, 87u8,
                            125u8,
                        ],
                    )
                }
                #[doc = "Force there to be no new eras indefinitely."]
                #[doc = ""]
                #[doc = "The dispatch origin must be Root."]
                #[doc = ""]
                #[doc = "# Warning"]
                #[doc = ""]
                #[doc = "The election process starts multiple blocks before the end of the era."]
                #[doc = "Thus the election process may be ongoing when this is called. In this case the"]
                #[doc = "election will continue until the next era is triggered."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- No arguments."]
                #[doc = "- Weight: O(1)"]
                #[doc = "- Write: ForceEra"]
                #[doc = "# </weight>"]
                pub fn force_no_eras(&self) -> ::subxt::tx::StaticTxPayload<ForceNoEras> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "force_no_eras",
                        ForceNoEras {},
                        [
                            16u8, 81u8, 207u8, 168u8, 23u8, 236u8, 11u8, 75u8, 141u8, 107u8, 92u8,
                            2u8, 53u8, 111u8, 252u8, 116u8, 91u8, 120u8, 75u8, 24u8, 125u8, 53u8,
                            9u8, 28u8, 242u8, 87u8, 245u8, 55u8, 40u8, 103u8, 151u8, 178u8,
                        ],
                    )
                }
                #[doc = "Force there to be a new era at the end of the next session. After this, it will be"]
                #[doc = "reset to normal (non-forced) behaviour."]
                #[doc = ""]
                #[doc = "The dispatch origin must be Root."]
                #[doc = ""]
                #[doc = "# Warning"]
                #[doc = ""]
                #[doc = "The election process starts multiple blocks before the end of the era."]
                #[doc = "If this is called just before a new era is triggered, the election process may not"]
                #[doc = "have enough blocks to get a result."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- No arguments."]
                #[doc = "- Weight: O(1)"]
                #[doc = "- Write ForceEra"]
                #[doc = "# </weight>"]
                pub fn force_new_era(&self) -> ::subxt::tx::StaticTxPayload<ForceNewEra> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "force_new_era",
                        ForceNewEra {},
                        [
                            230u8, 242u8, 169u8, 196u8, 78u8, 145u8, 24u8, 191u8, 113u8, 68u8, 5u8,
                            138u8, 48u8, 51u8, 109u8, 126u8, 73u8, 136u8, 162u8, 158u8, 174u8,
                            201u8, 213u8, 230u8, 215u8, 44u8, 200u8, 32u8, 75u8, 27u8, 23u8, 254u8,
                        ],
                    )
                }
                #[doc = "Set the validators who cannot be slashed (if any)."]
                #[doc = ""]
                #[doc = "The dispatch origin must be Root."]
                pub fn set_invulnerables(
                    &self,
                    invulnerables: ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::tx::StaticTxPayload<SetInvulnerables> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "set_invulnerables",
                        SetInvulnerables { invulnerables },
                        [
                            2u8, 148u8, 221u8, 111u8, 153u8, 48u8, 222u8, 36u8, 228u8, 84u8, 18u8,
                            35u8, 168u8, 239u8, 53u8, 245u8, 27u8, 76u8, 18u8, 203u8, 206u8, 9u8,
                            8u8, 81u8, 35u8, 224u8, 22u8, 133u8, 58u8, 99u8, 103u8, 39u8,
                        ],
                    )
                }
                #[doc = "Force a current staker to become completely unstaked, immediately."]
                #[doc = ""]
                #[doc = "The dispatch origin must be Root."]
                pub fn force_unstake(
                    &self,
                    stash: ::subxt::ext::sp_core::crypto::AccountId32,
                    num_slashing_spans: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<ForceUnstake> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "force_unstake",
                        ForceUnstake {
                            stash,
                            num_slashing_spans,
                        },
                        [
                            94u8, 247u8, 238u8, 47u8, 250u8, 6u8, 96u8, 175u8, 173u8, 123u8, 161u8,
                            187u8, 162u8, 214u8, 176u8, 233u8, 33u8, 33u8, 167u8, 239u8, 40u8,
                            223u8, 19u8, 131u8, 230u8, 39u8, 175u8, 200u8, 36u8, 182u8, 76u8,
                            207u8,
                        ],
                    )
                }
                #[doc = "Force there to be a new era at the end of sessions indefinitely."]
                #[doc = ""]
                #[doc = "The dispatch origin must be Root."]
                #[doc = ""]
                #[doc = "# Warning"]
                #[doc = ""]
                #[doc = "The election process starts multiple blocks before the end of the era."]
                #[doc = "If this is called just before a new era is triggered, the election process may not"]
                #[doc = "have enough blocks to get a result."]
                pub fn force_new_era_always(
                    &self,
                ) -> ::subxt::tx::StaticTxPayload<ForceNewEraAlways> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "force_new_era_always",
                        ForceNewEraAlways {},
                        [
                            179u8, 118u8, 189u8, 54u8, 248u8, 141u8, 207u8, 142u8, 80u8, 37u8,
                            241u8, 185u8, 138u8, 254u8, 117u8, 147u8, 225u8, 118u8, 34u8, 177u8,
                            197u8, 158u8, 8u8, 82u8, 202u8, 108u8, 208u8, 26u8, 64u8, 33u8, 74u8,
                            43u8,
                        ],
                    )
                }
                #[doc = "Cancel enactment of a deferred slash."]
                #[doc = ""]
                #[doc = "Can be called by the `T::AdminOrigin`."]
                #[doc = ""]
                #[doc = "Parameters: era and indices of the slashes for that era to kill."]
                pub fn cancel_deferred_slash(
                    &self,
                    era: ::core::primitive::u32,
                    slash_indices: ::std::vec::Vec<::core::primitive::u32>,
                ) -> ::subxt::tx::StaticTxPayload<CancelDeferredSlash> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "cancel_deferred_slash",
                        CancelDeferredSlash { era, slash_indices },
                        [
                            120u8, 57u8, 162u8, 105u8, 91u8, 250u8, 129u8, 240u8, 110u8, 234u8,
                            170u8, 98u8, 164u8, 65u8, 106u8, 101u8, 19u8, 88u8, 146u8, 210u8,
                            171u8, 44u8, 37u8, 50u8, 65u8, 178u8, 37u8, 223u8, 239u8, 197u8, 116u8,
                            168u8,
                        ],
                    )
                }
                #[doc = "Pay out all the stakers behind a single validator for a single era."]
                #[doc = ""]
                #[doc = "- `validator_stash` is the stash account of the validator. Their nominators, up to"]
                #[doc = "  `T::MaxNominatorRewardedPerValidator`, will also receive their rewards."]
                #[doc = "- `era` may be any era between `[current_era - history_depth; current_era]`."]
                #[doc = ""]
                #[doc = "The origin of this call must be _Signed_. Any account can call this function, even if"]
                #[doc = "it is not one of the stakers."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- Time complexity: at most O(MaxNominatorRewardedPerValidator)."]
                #[doc = "- Contains a limited number of reads and writes."]
                #[doc = "-----------"]
                #[doc = "N is the Number of payouts for the validator (including the validator)"]
                #[doc = "Weight:"]
                #[doc = "- Reward Destination Staked: O(N)"]
                #[doc = "- Reward Destination Controller (Creating): O(N)"]
                #[doc = ""]
                #[doc = "  NOTE: weights are assuming that payouts are made to alive stash account (Staked)."]
                #[doc = "  Paying even a dead controller is cheaper weight-wise. We don't do any refunds here."]
                #[doc = "# </weight>"]
                pub fn payout_stakers(
                    &self,
                    validator_stash: ::subxt::ext::sp_core::crypto::AccountId32,
                    era: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<PayoutStakers> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "payout_stakers",
                        PayoutStakers {
                            validator_stash,
                            era,
                        },
                        [
                            184u8, 194u8, 33u8, 118u8, 7u8, 203u8, 89u8, 119u8, 214u8, 76u8, 178u8,
                            20u8, 82u8, 111u8, 57u8, 132u8, 212u8, 43u8, 232u8, 91u8, 252u8, 49u8,
                            42u8, 115u8, 1u8, 181u8, 154u8, 207u8, 144u8, 206u8, 205u8, 33u8,
                        ],
                    )
                }
                #[doc = "Rebond a portion of the stash scheduled to be unlocked."]
                #[doc = ""]
                #[doc = "The dispatch origin must be signed by the controller."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- Time complexity: O(L), where L is unlocking chunks"]
                #[doc = "- Bounded by `MaxUnlockingChunks`."]
                #[doc = "- Storage changes: Can't increase storage, only decrease it."]
                #[doc = "# </weight>"]
                pub fn rebond(
                    &self,
                    value: ::core::primitive::u128,
                ) -> ::subxt::tx::StaticTxPayload<Rebond> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "rebond",
                        Rebond { value },
                        [
                            25u8, 22u8, 191u8, 172u8, 133u8, 101u8, 139u8, 102u8, 134u8, 16u8,
                            136u8, 56u8, 137u8, 162u8, 4u8, 253u8, 196u8, 30u8, 234u8, 49u8, 102u8,
                            68u8, 145u8, 96u8, 148u8, 219u8, 162u8, 17u8, 177u8, 184u8, 34u8,
                            113u8,
                        ],
                    )
                }
                #[doc = "Remove all data structures concerning a staker/stash once it is at a state where it can"]
                #[doc = "be considered `dust` in the staking system. The requirements are:"]
                #[doc = ""]
                #[doc = "1. the `total_balance` of the stash is below existential deposit."]
                #[doc = "2. or, the `ledger.total` of the stash is below existential deposit."]
                #[doc = ""]
                #[doc = "The former can happen in cases like a slash; the latter when a fully unbonded account"]
                #[doc = "is still receiving staking rewards in `RewardDestination::Staked`."]
                #[doc = ""]
                #[doc = "It can be called by anyone, as long as `stash` meets the above requirements."]
                #[doc = ""]
                #[doc = "Refunds the transaction fees upon successful execution."]
                pub fn reap_stash(
                    &self,
                    stash: ::subxt::ext::sp_core::crypto::AccountId32,
                    num_slashing_spans: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<ReapStash> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "reap_stash",
                        ReapStash {
                            stash,
                            num_slashing_spans,
                        },
                        [
                            34u8, 168u8, 120u8, 161u8, 95u8, 199u8, 106u8, 233u8, 61u8, 240u8,
                            166u8, 31u8, 183u8, 165u8, 158u8, 179u8, 32u8, 130u8, 27u8, 164u8,
                            112u8, 44u8, 14u8, 125u8, 227u8, 87u8, 70u8, 203u8, 194u8, 24u8, 212u8,
                            177u8,
                        ],
                    )
                }
                #[doc = "Remove the given nominations from the calling validator."]
                #[doc = ""]
                #[doc = "Effects will be felt at the beginning of the next era."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ by the controller, not the stash."]
                #[doc = ""]
                #[doc = "- `who`: A list of nominator stash accounts who are nominating this validator which"]
                #[doc = "  should no longer be nominating this validator."]
                #[doc = ""]
                #[doc = "Note: Making this call only makes sense if you first set the validator preferences to"]
                #[doc = "block any further nominations."]
                pub fn kick(
                    &self,
                    who: ::std::vec::Vec<
                        ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                    >,
                ) -> ::subxt::tx::StaticTxPayload<Kick> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "kick",
                        Kick { who },
                        [
                            32u8, 26u8, 202u8, 6u8, 186u8, 180u8, 58u8, 121u8, 185u8, 208u8, 123u8,
                            10u8, 53u8, 179u8, 167u8, 203u8, 96u8, 229u8, 7u8, 144u8, 231u8, 172u8,
                            145u8, 141u8, 162u8, 180u8, 212u8, 42u8, 34u8, 5u8, 199u8, 82u8,
                        ],
                    )
                }
                #[doc = "Update the various staking configurations ."]
                #[doc = ""]
                #[doc = "* `min_nominator_bond`: The minimum active bond needed to be a nominator."]
                #[doc = "* `min_validator_bond`: The minimum active bond needed to be a validator."]
                #[doc = "* `max_nominator_count`: The max number of users who can be a nominator at once. When"]
                #[doc = "  set to `None`, no limit is enforced."]
                #[doc = "* `max_validator_count`: The max number of users who can be a validator at once. When"]
                #[doc = "  set to `None`, no limit is enforced."]
                #[doc = "* `chill_threshold`: The ratio of `max_nominator_count` or `max_validator_count` which"]
                #[doc = "  should be filled in order for the `chill_other` transaction to work."]
                #[doc = "* `min_commission`: The minimum amount of commission that each validators must maintain."]
                #[doc = "  This is checked only upon calling `validate`. Existing validators are not affected."]
                #[doc = ""]
                #[doc = "RuntimeOrigin must be Root to call this function."]
                #[doc = ""]
                #[doc = "NOTE: Existing nominators and validators will not be affected by this update."]
                #[doc = "to kick people under the new limits, `chill_other` should be called."]
                pub fn set_staking_configs(
                    &self,
                    min_nominator_bond: runtime_types::pallet_staking::pallet::pallet::ConfigOp<
                        ::core::primitive::u128,
                    >,
                    min_validator_bond: runtime_types::pallet_staking::pallet::pallet::ConfigOp<
                        ::core::primitive::u128,
                    >,
                    max_nominator_count: runtime_types::pallet_staking::pallet::pallet::ConfigOp<
                        ::core::primitive::u32,
                    >,
                    max_validator_count: runtime_types::pallet_staking::pallet::pallet::ConfigOp<
                        ::core::primitive::u32,
                    >,
                    chill_threshold: runtime_types::pallet_staking::pallet::pallet::ConfigOp<
                        runtime_types::sp_arithmetic::per_things::Percent,
                    >,
                    min_commission: runtime_types::pallet_staking::pallet::pallet::ConfigOp<
                        runtime_types::sp_arithmetic::per_things::Perbill,
                    >,
                ) -> ::subxt::tx::StaticTxPayload<SetStakingConfigs> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "set_staking_configs",
                        SetStakingConfigs {
                            min_nominator_bond,
                            min_validator_bond,
                            max_nominator_count,
                            max_validator_count,
                            chill_threshold,
                            min_commission,
                        },
                        [
                            176u8, 168u8, 155u8, 176u8, 27u8, 79u8, 223u8, 92u8, 88u8, 93u8, 223u8,
                            69u8, 179u8, 250u8, 138u8, 138u8, 87u8, 220u8, 36u8, 3u8, 126u8, 213u8,
                            16u8, 68u8, 3u8, 16u8, 218u8, 151u8, 98u8, 169u8, 217u8, 75u8,
                        ],
                    )
                }
                #[doc = "Declare a `controller` to stop participating as either a validator or nominator."]
                #[doc = ""]
                #[doc = "Effects will be felt at the beginning of the next era."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_, but can be called by anyone."]
                #[doc = ""]
                #[doc = "If the caller is the same as the controller being targeted, then no further checks are"]
                #[doc = "enforced, and this function behaves just like `chill`."]
                #[doc = ""]
                #[doc = "If the caller is different than the controller being targeted, the following conditions"]
                #[doc = "must be met:"]
                #[doc = ""]
                #[doc = "* `controller` must belong to a nominator who has become non-decodable,"]
                #[doc = ""]
                #[doc = "Or:"]
                #[doc = ""]
                #[doc = "* A `ChillThreshold` must be set and checked which defines how close to the max"]
                #[doc = "  nominators or validators we must reach before users can start chilling one-another."]
                #[doc = "* A `MaxNominatorCount` and `MaxValidatorCount` must be set which is used to determine"]
                #[doc = "  how close we are to the threshold."]
                #[doc = "* A `MinNominatorBond` and `MinValidatorBond` must be set and checked, which determines"]
                #[doc = "  if this is a person that should be chilled because they have not met the threshold"]
                #[doc = "  bond required."]
                #[doc = ""]
                #[doc = "This can be helpful if bond requirements are updated, and we need to remove old users"]
                #[doc = "who do not satisfy these requirements."]
                pub fn chill_other(
                    &self,
                    controller: ::subxt::ext::sp_core::crypto::AccountId32,
                ) -> ::subxt::tx::StaticTxPayload<ChillOther> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "chill_other",
                        ChillOther { controller },
                        [
                            140u8, 98u8, 4u8, 203u8, 91u8, 131u8, 123u8, 119u8, 169u8, 47u8, 188u8,
                            23u8, 205u8, 170u8, 82u8, 220u8, 166u8, 170u8, 135u8, 176u8, 68u8,
                            228u8, 14u8, 67u8, 42u8, 52u8, 140u8, 231u8, 62u8, 167u8, 80u8, 173u8,
                        ],
                    )
                }
                #[doc = "Force a validator to have at least the minimum commission. This will not affect a"]
                #[doc = "validator who already has a commission greater than or equal to the minimum. Any account"]
                #[doc = "can call this."]
                pub fn force_apply_min_commission(
                    &self,
                    validator_stash: ::subxt::ext::sp_core::crypto::AccountId32,
                ) -> ::subxt::tx::StaticTxPayload<ForceApplyMinCommission> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "force_apply_min_commission",
                        ForceApplyMinCommission { validator_stash },
                        [
                            136u8, 163u8, 85u8, 134u8, 240u8, 247u8, 183u8, 227u8, 226u8, 202u8,
                            102u8, 186u8, 138u8, 119u8, 78u8, 123u8, 229u8, 135u8, 129u8, 241u8,
                            119u8, 106u8, 41u8, 182u8, 121u8, 181u8, 242u8, 175u8, 74u8, 207u8,
                            64u8, 106u8,
                        ],
                    )
                }
                #[doc = "Sets the minimum amount of commission that each validators must maintain."]
                #[doc = ""]
                #[doc = "This call has lower privilege requirements than `set_staking_config` and can be called"]
                #[doc = "by the `T::AdminOrigin`. Root can always call this."]
                pub fn set_min_commission(
                    &self,
                    new: runtime_types::sp_arithmetic::per_things::Perbill,
                ) -> ::subxt::tx::StaticTxPayload<SetMinCommission> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Staking",
                        "set_min_commission",
                        SetMinCommission { new },
                        [
                            62u8, 139u8, 175u8, 245u8, 212u8, 113u8, 117u8, 130u8, 191u8, 173u8,
                            78u8, 97u8, 19u8, 104u8, 185u8, 207u8, 201u8, 14u8, 200u8, 208u8,
                            184u8, 195u8, 242u8, 175u8, 158u8, 156u8, 51u8, 58u8, 118u8, 154u8,
                            68u8, 221u8,
                        ],
                    )
                }
            }
        }
        #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
        pub type Event = runtime_types::pallet_staking::pallet::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "The era payout has been set; the first balance is the validator-payout; the second is"]
            #[doc = "the remainder from the maximum amount of reward."]
            pub struct EraPaid {
                pub era_index: ::core::primitive::u32,
                pub validator_payout: ::core::primitive::u128,
                pub remainder: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for EraPaid {
                const PALLET: &'static str = "Staking";
                const EVENT: &'static str = "EraPaid";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "The nominator has been rewarded by this amount."]
            pub struct Rewarded {
                pub stash: ::subxt::ext::sp_core::crypto::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Rewarded {
                const PALLET: &'static str = "Staking";
                const EVENT: &'static str = "Rewarded";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A staker (validator or nominator) has been slashed by the given amount."]
            pub struct Slashed {
                pub staker: ::subxt::ext::sp_core::crypto::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Slashed {
                const PALLET: &'static str = "Staking";
                const EVENT: &'static str = "Slashed";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A slash for the given validator, for the given percentage of their stake, at the given"]
            #[doc = "era as been reported."]
            pub struct SlashReported {
                pub validator: ::subxt::ext::sp_core::crypto::AccountId32,
                pub fraction: runtime_types::sp_arithmetic::per_things::Perbill,
                pub slash_era: ::core::primitive::u32,
            }
            impl ::subxt::events::StaticEvent for SlashReported {
                const PALLET: &'static str = "Staking";
                const EVENT: &'static str = "SlashReported";
            }
            #[derive(
                :: subxt :: ext :: codec :: CompactAs,
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "An old slashing report from a prior era was discarded because it could"]
            #[doc = "not be processed."]
            pub struct OldSlashingReportDiscarded {
                pub session_index: ::core::primitive::u32,
            }
            impl ::subxt::events::StaticEvent for OldSlashingReportDiscarded {
                const PALLET: &'static str = "Staking";
                const EVENT: &'static str = "OldSlashingReportDiscarded";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A new set of stakers was elected."]
            pub struct StakersElected;
            impl ::subxt::events::StaticEvent for StakersElected {
                const PALLET: &'static str = "Staking";
                const EVENT: &'static str = "StakersElected";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "An account has bonded this amount. \\[stash, amount\\]"]
            #[doc = ""]
            #[doc = "NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,"]
            #[doc = "it will not be emitted for staking rewards when they are added to stake."]
            pub struct Bonded {
                pub stash: ::subxt::ext::sp_core::crypto::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Bonded {
                const PALLET: &'static str = "Staking";
                const EVENT: &'static str = "Bonded";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "An account has unbonded this amount."]
            pub struct Unbonded {
                pub stash: ::subxt::ext::sp_core::crypto::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Unbonded {
                const PALLET: &'static str = "Staking";
                const EVENT: &'static str = "Unbonded";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`"]
            #[doc = "from the unlocking queue."]
            pub struct Withdrawn {
                pub stash: ::subxt::ext::sp_core::crypto::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Withdrawn {
                const PALLET: &'static str = "Staking";
                const EVENT: &'static str = "Withdrawn";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A nominator has been kicked from a validator."]
            pub struct Kicked {
                pub nominator: ::subxt::ext::sp_core::crypto::AccountId32,
                pub stash: ::subxt::ext::sp_core::crypto::AccountId32,
            }
            impl ::subxt::events::StaticEvent for Kicked {
                const PALLET: &'static str = "Staking";
                const EVENT: &'static str = "Kicked";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "The election failed. No new era is planned."]
            pub struct StakingElectionFailed;
            impl ::subxt::events::StaticEvent for StakingElectionFailed {
                const PALLET: &'static str = "Staking";
                const EVENT: &'static str = "StakingElectionFailed";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "An account has stopped participating as either a validator or nominator."]
            pub struct Chilled {
                pub stash: ::subxt::ext::sp_core::crypto::AccountId32,
            }
            impl ::subxt::events::StaticEvent for Chilled {
                const PALLET: &'static str = "Staking";
                const EVENT: &'static str = "Chilled";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "The stakers' rewards are getting paid."]
            pub struct PayoutStarted {
                pub era_index: ::core::primitive::u32,
                pub validator_stash: ::subxt::ext::sp_core::crypto::AccountId32,
            }
            impl ::subxt::events::StaticEvent for PayoutStarted {
                const PALLET: &'static str = "Staking";
                const EVENT: &'static str = "PayoutStarted";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A validator has set their preferences."]
            pub struct ValidatorPrefsSet {
                pub stash: ::subxt::ext::sp_core::crypto::AccountId32,
                pub prefs: runtime_types::pallet_staking::ValidatorPrefs,
            }
            impl ::subxt::events::StaticEvent for ValidatorPrefsSet {
                const PALLET: &'static str = "Staking";
                const EVENT: &'static str = "ValidatorPrefsSet";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A new force era mode was set."]
            pub struct ForceEra {
                pub mode: runtime_types::pallet_staking::Forcing,
            }
            impl ::subxt::events::StaticEvent for ForceEra {
                const PALLET: &'static str = "Staking";
                const EVENT: &'static str = "ForceEra";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                #[doc = " The ideal number of active validators."]
                pub fn validator_count(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ValidatorCount",
                        vec![],
                        [
                            245u8, 75u8, 214u8, 110u8, 66u8, 164u8, 86u8, 206u8, 69u8, 89u8, 12u8,
                            111u8, 117u8, 16u8, 228u8, 184u8, 207u8, 6u8, 0u8, 126u8, 221u8, 67u8,
                            125u8, 218u8, 188u8, 245u8, 156u8, 188u8, 34u8, 85u8, 208u8, 197u8,
                        ],
                    )
                }
                #[doc = " Minimum number of staking participants before emergency conditions are imposed."]
                pub fn minimum_validator_count(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "MinimumValidatorCount",
                        vec![],
                        [
                            82u8, 95u8, 128u8, 55u8, 136u8, 134u8, 71u8, 117u8, 135u8, 76u8, 44u8,
                            46u8, 174u8, 34u8, 170u8, 228u8, 175u8, 1u8, 234u8, 162u8, 91u8, 252u8,
                            127u8, 68u8, 243u8, 241u8, 13u8, 107u8, 214u8, 70u8, 87u8, 249u8,
                        ],
                    )
                }
                #[doc = " Any validators that may never be slashed or forcibly kicked. It's a Vec since they're"]
                #[doc = " easy to initialize and the performance hit is minimal (we expect no more than four"]
                #[doc = " invulnerables) and restricted to testnets."]
                pub fn invulnerables(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "Invulnerables",
                        vec![],
                        [
                            77u8, 78u8, 63u8, 199u8, 150u8, 167u8, 135u8, 130u8, 192u8, 51u8,
                            202u8, 119u8, 68u8, 49u8, 241u8, 68u8, 82u8, 90u8, 226u8, 201u8, 96u8,
                            170u8, 21u8, 173u8, 236u8, 116u8, 148u8, 8u8, 174u8, 92u8, 7u8, 11u8,
                        ],
                    )
                }
                #[doc = " Map from all locked \"stash\" accounts to the controller account."]
                #[doc = ""]
                #[doc = " TWOX-NOTE: SAFE since `AccountId` is a secure hash."]
                pub fn bonded(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::subxt::ext::sp_core::crypto::AccountId32>,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "Bonded",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            35u8, 197u8, 156u8, 60u8, 22u8, 59u8, 103u8, 83u8, 77u8, 15u8, 118u8,
                            193u8, 155u8, 97u8, 229u8, 36u8, 119u8, 128u8, 224u8, 162u8, 21u8,
                            46u8, 199u8, 221u8, 15u8, 74u8, 59u8, 70u8, 77u8, 218u8, 73u8, 165u8,
                        ],
                    )
                }
                #[doc = " Map from all locked \"stash\" accounts to the controller account."]
                #[doc = ""]
                #[doc = " TWOX-NOTE: SAFE since `AccountId` is a secure hash."]
                pub fn bonded_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::subxt::ext::sp_core::crypto::AccountId32>,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "Bonded",
                        Vec::new(),
                        [
                            35u8, 197u8, 156u8, 60u8, 22u8, 59u8, 103u8, 83u8, 77u8, 15u8, 118u8,
                            193u8, 155u8, 97u8, 229u8, 36u8, 119u8, 128u8, 224u8, 162u8, 21u8,
                            46u8, 199u8, 221u8, 15u8, 74u8, 59u8, 70u8, 77u8, 218u8, 73u8, 165u8,
                        ],
                    )
                }
                #[doc = " The minimum active bond to become and maintain the role of a nominator."]
                pub fn min_nominator_bond(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "MinNominatorBond",
                        vec![],
                        [
                            187u8, 66u8, 149u8, 226u8, 72u8, 219u8, 57u8, 246u8, 102u8, 47u8, 71u8,
                            12u8, 219u8, 204u8, 127u8, 223u8, 58u8, 134u8, 81u8, 165u8, 200u8,
                            142u8, 196u8, 158u8, 26u8, 38u8, 165u8, 19u8, 91u8, 251u8, 119u8, 84u8,
                        ],
                    )
                }
                #[doc = " The minimum active bond to become and maintain the role of a validator."]
                pub fn min_validator_bond(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "MinValidatorBond",
                        vec![],
                        [
                            48u8, 105u8, 85u8, 178u8, 142u8, 208u8, 208u8, 19u8, 236u8, 130u8,
                            129u8, 169u8, 35u8, 245u8, 66u8, 182u8, 92u8, 20u8, 22u8, 109u8, 155u8,
                            174u8, 87u8, 118u8, 242u8, 216u8, 193u8, 154u8, 4u8, 5u8, 66u8, 56u8,
                        ],
                    )
                }
                #[doc = " The minimum active nominator stake of the last successful election."]
                pub fn minimum_active_stake(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "MinimumActiveStake",
                        vec![],
                        [
                            172u8, 190u8, 228u8, 47u8, 47u8, 192u8, 182u8, 59u8, 9u8, 18u8, 103u8,
                            46u8, 175u8, 54u8, 17u8, 79u8, 89u8, 107u8, 255u8, 200u8, 182u8, 107u8,
                            89u8, 157u8, 55u8, 16u8, 77u8, 46u8, 154u8, 169u8, 103u8, 151u8,
                        ],
                    )
                }
                #[doc = " The minimum amount of commission that validators can set."]
                #[doc = ""]
                #[doc = " If set to `0`, no limit exists."]
                pub fn min_commission(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_arithmetic::per_things::Perbill,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "MinCommission",
                        vec![],
                        [
                            61u8, 101u8, 69u8, 27u8, 220u8, 179u8, 5u8, 71u8, 66u8, 227u8, 84u8,
                            98u8, 18u8, 141u8, 183u8, 49u8, 98u8, 46u8, 123u8, 114u8, 198u8, 85u8,
                            15u8, 175u8, 243u8, 239u8, 133u8, 129u8, 146u8, 174u8, 254u8, 158u8,
                        ],
                    )
                }
                #[doc = " Map from all (unlocked) \"controller\" accounts to the info regarding the staking."]
                pub fn ledger(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::StakingLedger,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "Ledger",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Blake2_128Concat,
                        )],
                        [
                            31u8, 205u8, 3u8, 165u8, 22u8, 22u8, 62u8, 92u8, 33u8, 189u8, 124u8,
                            120u8, 177u8, 70u8, 27u8, 242u8, 188u8, 184u8, 204u8, 188u8, 242u8,
                            140u8, 128u8, 230u8, 85u8, 99u8, 181u8, 173u8, 67u8, 252u8, 37u8,
                            236u8,
                        ],
                    )
                }
                #[doc = " Map from all (unlocked) \"controller\" accounts to the info regarding the staking."]
                pub fn ledger_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::StakingLedger,
                    >,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "Ledger",
                        Vec::new(),
                        [
                            31u8, 205u8, 3u8, 165u8, 22u8, 22u8, 62u8, 92u8, 33u8, 189u8, 124u8,
                            120u8, 177u8, 70u8, 27u8, 242u8, 188u8, 184u8, 204u8, 188u8, 242u8,
                            140u8, 128u8, 230u8, 85u8, 99u8, 181u8, 173u8, 67u8, 252u8, 37u8,
                            236u8,
                        ],
                    )
                }
                #[doc = " Where the reward payment should be made. Keyed by stash."]
                #[doc = ""]
                #[doc = " TWOX-NOTE: SAFE since `AccountId` is a secure hash."]
                pub fn payee(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::RewardDestination<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "Payee",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            195u8, 125u8, 82u8, 213u8, 216u8, 64u8, 76u8, 63u8, 187u8, 163u8, 20u8,
                            230u8, 153u8, 13u8, 189u8, 232u8, 119u8, 118u8, 107u8, 17u8, 102u8,
                            245u8, 36u8, 42u8, 232u8, 137u8, 177u8, 165u8, 169u8, 246u8, 199u8,
                            57u8,
                        ],
                    )
                }
                #[doc = " Where the reward payment should be made. Keyed by stash."]
                #[doc = ""]
                #[doc = " TWOX-NOTE: SAFE since `AccountId` is a secure hash."]
                pub fn payee_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::RewardDestination<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                        >,
                    >,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "Payee",
                        Vec::new(),
                        [
                            195u8, 125u8, 82u8, 213u8, 216u8, 64u8, 76u8, 63u8, 187u8, 163u8, 20u8,
                            230u8, 153u8, 13u8, 189u8, 232u8, 119u8, 118u8, 107u8, 17u8, 102u8,
                            245u8, 36u8, 42u8, 232u8, 137u8, 177u8, 165u8, 169u8, 246u8, 199u8,
                            57u8,
                        ],
                    )
                }
                #[doc = " The map from (wannabe) validator stash key to the preferences of that validator."]
                #[doc = ""]
                #[doc = " TWOX-NOTE: SAFE since `AccountId` is a secure hash."]
                pub fn validators(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::ValidatorPrefs,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "Validators",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            80u8, 77u8, 66u8, 18u8, 197u8, 250u8, 41u8, 185u8, 43u8, 24u8, 149u8,
                            164u8, 208u8, 60u8, 144u8, 29u8, 251u8, 195u8, 236u8, 196u8, 108u8,
                            58u8, 80u8, 115u8, 246u8, 66u8, 226u8, 241u8, 201u8, 172u8, 229u8,
                            152u8,
                        ],
                    )
                }
                #[doc = " The map from (wannabe) validator stash key to the preferences of that validator."]
                #[doc = ""]
                #[doc = " TWOX-NOTE: SAFE since `AccountId` is a secure hash."]
                pub fn validators_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::ValidatorPrefs,
                    >,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "Validators",
                        Vec::new(),
                        [
                            80u8, 77u8, 66u8, 18u8, 197u8, 250u8, 41u8, 185u8, 43u8, 24u8, 149u8,
                            164u8, 208u8, 60u8, 144u8, 29u8, 251u8, 195u8, 236u8, 196u8, 108u8,
                            58u8, 80u8, 115u8, 246u8, 66u8, 226u8, 241u8, 201u8, 172u8, 229u8,
                            152u8,
                        ],
                    )
                }
                #[doc = "Counter for the related counted storage map"]
                pub fn counter_for_validators(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "CounterForValidators",
                        vec![],
                        [
                            139u8, 25u8, 223u8, 6u8, 160u8, 239u8, 212u8, 85u8, 36u8, 185u8, 69u8,
                            63u8, 21u8, 156u8, 144u8, 241u8, 112u8, 85u8, 49u8, 78u8, 88u8, 11u8,
                            8u8, 48u8, 118u8, 34u8, 62u8, 159u8, 239u8, 122u8, 90u8, 45u8,
                        ],
                    )
                }
                #[doc = " The maximum validator count before we stop allowing new validators to join."]
                #[doc = ""]
                #[doc = " When this value is not set, no limits are enforced."]
                pub fn max_validators_count(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "MaxValidatorsCount",
                        vec![],
                        [
                            250u8, 62u8, 16u8, 68u8, 192u8, 216u8, 236u8, 211u8, 217u8, 9u8, 213u8,
                            49u8, 41u8, 37u8, 58u8, 62u8, 131u8, 112u8, 64u8, 26u8, 133u8, 7u8,
                            130u8, 1u8, 71u8, 158u8, 14u8, 55u8, 169u8, 239u8, 223u8, 245u8,
                        ],
                    )
                }
                #[doc = " The map from nominator stash key to their nomination preferences, namely the validators that"]
                #[doc = " they wish to support."]
                #[doc = ""]
                #[doc = " Note that the keys of this storage map might become non-decodable in case the"]
                #[doc = " [`Config::MaxNominations`] configuration is decreased. In this rare case, these nominators"]
                #[doc = " are still existent in storage, their key is correct and retrievable (i.e. `contains_key`"]
                #[doc = " indicates that they exist), but their value cannot be decoded. Therefore, the non-decodable"]
                #[doc = " nominators will effectively not-exist, until they re-submit their preferences such that it"]
                #[doc = " is within the bounds of the newly set `Config::MaxNominations`."]
                #[doc = ""]
                #[doc = " This implies that `::iter_keys().count()` and `::iter().count()` might return different"]
                #[doc = " values for this map. Moreover, the main `::count()` is aligned with the former, namely the"]
                #[doc = " number of keys that exist."]
                #[doc = ""]
                #[doc = " Lastly, if any of the nominators become non-decodable, they can be chilled immediately via"]
                #[doc = " [`Call::chill_other`] dispatchable by anyone."]
                #[doc = ""]
                #[doc = " TWOX-NOTE: SAFE since `AccountId` is a secure hash."]
                pub fn nominators(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::pallet_staking::Nominations>,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "Nominators",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            1u8, 154u8, 55u8, 170u8, 215u8, 64u8, 56u8, 83u8, 254u8, 19u8, 152u8,
                            85u8, 164u8, 171u8, 206u8, 129u8, 184u8, 45u8, 221u8, 181u8, 229u8,
                            133u8, 200u8, 231u8, 16u8, 146u8, 247u8, 21u8, 77u8, 122u8, 165u8,
                            134u8,
                        ],
                    )
                }
                #[doc = " The map from nominator stash key to their nomination preferences, namely the validators that"]
                #[doc = " they wish to support."]
                #[doc = ""]
                #[doc = " Note that the keys of this storage map might become non-decodable in case the"]
                #[doc = " [`Config::MaxNominations`] configuration is decreased. In this rare case, these nominators"]
                #[doc = " are still existent in storage, their key is correct and retrievable (i.e. `contains_key`"]
                #[doc = " indicates that they exist), but their value cannot be decoded. Therefore, the non-decodable"]
                #[doc = " nominators will effectively not-exist, until they re-submit their preferences such that it"]
                #[doc = " is within the bounds of the newly set `Config::MaxNominations`."]
                #[doc = ""]
                #[doc = " This implies that `::iter_keys().count()` and `::iter().count()` might return different"]
                #[doc = " values for this map. Moreover, the main `::count()` is aligned with the former, namely the"]
                #[doc = " number of keys that exist."]
                #[doc = ""]
                #[doc = " Lastly, if any of the nominators become non-decodable, they can be chilled immediately via"]
                #[doc = " [`Call::chill_other`] dispatchable by anyone."]
                #[doc = ""]
                #[doc = " TWOX-NOTE: SAFE since `AccountId` is a secure hash."]
                pub fn nominators_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::pallet_staking::Nominations>,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "Nominators",
                        Vec::new(),
                        [
                            1u8, 154u8, 55u8, 170u8, 215u8, 64u8, 56u8, 83u8, 254u8, 19u8, 152u8,
                            85u8, 164u8, 171u8, 206u8, 129u8, 184u8, 45u8, 221u8, 181u8, 229u8,
                            133u8, 200u8, 231u8, 16u8, 146u8, 247u8, 21u8, 77u8, 122u8, 165u8,
                            134u8,
                        ],
                    )
                }
                #[doc = "Counter for the related counted storage map"]
                pub fn counter_for_nominators(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "CounterForNominators",
                        vec![],
                        [
                            31u8, 94u8, 130u8, 138u8, 75u8, 8u8, 38u8, 162u8, 181u8, 5u8, 125u8,
                            116u8, 9u8, 51u8, 22u8, 234u8, 40u8, 117u8, 215u8, 46u8, 82u8, 117u8,
                            225u8, 1u8, 9u8, 208u8, 83u8, 63u8, 39u8, 187u8, 207u8, 191u8,
                        ],
                    )
                }
                #[doc = " The maximum nominator count before we stop allowing new validators to join."]
                #[doc = ""]
                #[doc = " When this value is not set, no limits are enforced."]
                pub fn max_nominators_count(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "MaxNominatorsCount",
                        vec![],
                        [
                            180u8, 190u8, 180u8, 66u8, 235u8, 173u8, 76u8, 160u8, 197u8, 92u8,
                            96u8, 165u8, 220u8, 188u8, 32u8, 119u8, 3u8, 73u8, 86u8, 49u8, 104u8,
                            17u8, 186u8, 98u8, 221u8, 175u8, 109u8, 254u8, 207u8, 245u8, 125u8,
                            179u8,
                        ],
                    )
                }
                #[doc = " The current era index."]
                #[doc = ""]
                #[doc = " This is the latest planned era, depending on how the Session pallet queues the validator"]
                #[doc = " set, it might be active or not."]
                pub fn current_era(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "CurrentEra",
                        vec![],
                        [
                            105u8, 150u8, 49u8, 122u8, 4u8, 78u8, 8u8, 121u8, 34u8, 136u8, 157u8,
                            227u8, 59u8, 139u8, 7u8, 253u8, 7u8, 10u8, 117u8, 71u8, 240u8, 74u8,
                            86u8, 36u8, 198u8, 37u8, 153u8, 93u8, 196u8, 22u8, 192u8, 243u8,
                        ],
                    )
                }
                #[doc = " The active era information, it holds index and start."]
                #[doc = ""]
                #[doc = " The active era is the era being currently rewarded. Validator set of this era must be"]
                #[doc = " equal to [`SessionInterface::validators`]."]
                pub fn active_era(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::ActiveEraInfo,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ActiveEra",
                        vec![],
                        [
                            15u8, 112u8, 251u8, 183u8, 108u8, 61u8, 28u8, 71u8, 44u8, 150u8, 162u8,
                            4u8, 143u8, 121u8, 11u8, 37u8, 83u8, 29u8, 193u8, 21u8, 210u8, 116u8,
                            190u8, 236u8, 213u8, 235u8, 49u8, 97u8, 189u8, 142u8, 251u8, 124u8,
                        ],
                    )
                }
                #[doc = " The session index at which the era start for the last `HISTORY_DEPTH` eras."]
                #[doc = ""]
                #[doc = " Note: This tracks the starting session (i.e. session index when era start being active)"]
                #[doc = " for the eras in `[CurrentEra - HISTORY_DEPTH, CurrentEra]`."]
                pub fn eras_start_session_index(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ErasStartSessionIndex",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            92u8, 157u8, 168u8, 144u8, 132u8, 3u8, 212u8, 80u8, 230u8, 229u8,
                            251u8, 218u8, 97u8, 55u8, 79u8, 100u8, 163u8, 91u8, 32u8, 246u8, 122u8,
                            78u8, 149u8, 214u8, 103u8, 249u8, 119u8, 20u8, 101u8, 116u8, 110u8,
                            185u8,
                        ],
                    )
                }
                #[doc = " The session index at which the era start for the last `HISTORY_DEPTH` eras."]
                #[doc = ""]
                #[doc = " Note: This tracks the starting session (i.e. session index when era start being active)"]
                #[doc = " for the eras in `[CurrentEra - HISTORY_DEPTH, CurrentEra]`."]
                pub fn eras_start_session_index_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ErasStartSessionIndex",
                        Vec::new(),
                        [
                            92u8, 157u8, 168u8, 144u8, 132u8, 3u8, 212u8, 80u8, 230u8, 229u8,
                            251u8, 218u8, 97u8, 55u8, 79u8, 100u8, 163u8, 91u8, 32u8, 246u8, 122u8,
                            78u8, 149u8, 214u8, 103u8, 249u8, 119u8, 20u8, 101u8, 116u8, 110u8,
                            185u8,
                        ],
                    )
                }
                #[doc = " Exposure of validator at era."]
                #[doc = ""]
                #[doc = " This is keyed first by the era index to allow bulk deletion and then the stash account."]
                #[doc = ""]
                #[doc = " Is it removed after `HISTORY_DEPTH` eras."]
                #[doc = " If stakers hasn't been set or has been removed then empty exposure is returned."]
                pub fn eras_stakers(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                    _1: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::Exposure<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            ::core::primitive::u128,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ErasStakers",
                        vec![
                            ::subxt::storage::address::StorageMapKey::new(
                                _0.borrow(),
                                ::subxt::storage::address::StorageHasher::Twox64Concat,
                            ),
                            ::subxt::storage::address::StorageMapKey::new(
                                _1.borrow(),
                                ::subxt::storage::address::StorageHasher::Twox64Concat,
                            ),
                        ],
                        [
                            192u8, 50u8, 152u8, 151u8, 92u8, 180u8, 206u8, 15u8, 139u8, 210u8,
                            128u8, 65u8, 92u8, 253u8, 43u8, 35u8, 139u8, 171u8, 73u8, 185u8, 32u8,
                            78u8, 20u8, 197u8, 154u8, 90u8, 233u8, 231u8, 23u8, 22u8, 187u8, 156u8,
                        ],
                    )
                }
                #[doc = " Exposure of validator at era."]
                #[doc = ""]
                #[doc = " This is keyed first by the era index to allow bulk deletion and then the stash account."]
                #[doc = ""]
                #[doc = " Is it removed after `HISTORY_DEPTH` eras."]
                #[doc = " If stakers hasn't been set or has been removed then empty exposure is returned."]
                pub fn eras_stakers_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::Exposure<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            ::core::primitive::u128,
                        >,
                    >,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ErasStakers",
                        Vec::new(),
                        [
                            192u8, 50u8, 152u8, 151u8, 92u8, 180u8, 206u8, 15u8, 139u8, 210u8,
                            128u8, 65u8, 92u8, 253u8, 43u8, 35u8, 139u8, 171u8, 73u8, 185u8, 32u8,
                            78u8, 20u8, 197u8, 154u8, 90u8, 233u8, 231u8, 23u8, 22u8, 187u8, 156u8,
                        ],
                    )
                }
                #[doc = " Clipped Exposure of validator at era."]
                #[doc = ""]
                #[doc = " This is similar to [`ErasStakers`] but number of nominators exposed is reduced to the"]
                #[doc = " `T::MaxNominatorRewardedPerValidator` biggest stakers."]
                #[doc = " (Note: the field `total` and `own` of the exposure remains unchanged)."]
                #[doc = " This is used to limit the i/o cost for the nominator payout."]
                #[doc = ""]
                #[doc = " This is keyed fist by the era index to allow bulk deletion and then the stash account."]
                #[doc = ""]
                #[doc = " Is it removed after `HISTORY_DEPTH` eras."]
                #[doc = " If stakers hasn't been set or has been removed then empty exposure is returned."]
                pub fn eras_stakers_clipped(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                    _1: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::Exposure<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            ::core::primitive::u128,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ErasStakersClipped",
                        vec![
                            ::subxt::storage::address::StorageMapKey::new(
                                _0.borrow(),
                                ::subxt::storage::address::StorageHasher::Twox64Concat,
                            ),
                            ::subxt::storage::address::StorageMapKey::new(
                                _1.borrow(),
                                ::subxt::storage::address::StorageHasher::Twox64Concat,
                            ),
                        ],
                        [
                            43u8, 159u8, 113u8, 223u8, 122u8, 169u8, 98u8, 153u8, 26u8, 55u8, 71u8,
                            119u8, 174u8, 48u8, 158u8, 45u8, 214u8, 26u8, 136u8, 215u8, 46u8,
                            161u8, 185u8, 17u8, 174u8, 204u8, 206u8, 246u8, 49u8, 87u8, 134u8,
                            169u8,
                        ],
                    )
                }
                #[doc = " Clipped Exposure of validator at era."]
                #[doc = ""]
                #[doc = " This is similar to [`ErasStakers`] but number of nominators exposed is reduced to the"]
                #[doc = " `T::MaxNominatorRewardedPerValidator` biggest stakers."]
                #[doc = " (Note: the field `total` and `own` of the exposure remains unchanged)."]
                #[doc = " This is used to limit the i/o cost for the nominator payout."]
                #[doc = ""]
                #[doc = " This is keyed fist by the era index to allow bulk deletion and then the stash account."]
                #[doc = ""]
                #[doc = " Is it removed after `HISTORY_DEPTH` eras."]
                #[doc = " If stakers hasn't been set or has been removed then empty exposure is returned."]
                pub fn eras_stakers_clipped_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::Exposure<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            ::core::primitive::u128,
                        >,
                    >,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ErasStakersClipped",
                        Vec::new(),
                        [
                            43u8, 159u8, 113u8, 223u8, 122u8, 169u8, 98u8, 153u8, 26u8, 55u8, 71u8,
                            119u8, 174u8, 48u8, 158u8, 45u8, 214u8, 26u8, 136u8, 215u8, 46u8,
                            161u8, 185u8, 17u8, 174u8, 204u8, 206u8, 246u8, 49u8, 87u8, 134u8,
                            169u8,
                        ],
                    )
                }
                #[doc = " Similar to `ErasStakers`, this holds the preferences of validators."]
                #[doc = ""]
                #[doc = " This is keyed first by the era index to allow bulk deletion and then the stash account."]
                #[doc = ""]
                #[doc = " Is it removed after `HISTORY_DEPTH` eras."]
                pub fn eras_validator_prefs(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                    _1: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::ValidatorPrefs,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ErasValidatorPrefs",
                        vec![
                            ::subxt::storage::address::StorageMapKey::new(
                                _0.borrow(),
                                ::subxt::storage::address::StorageHasher::Twox64Concat,
                            ),
                            ::subxt::storage::address::StorageMapKey::new(
                                _1.borrow(),
                                ::subxt::storage::address::StorageHasher::Twox64Concat,
                            ),
                        ],
                        [
                            6u8, 196u8, 209u8, 138u8, 252u8, 18u8, 203u8, 86u8, 129u8, 62u8, 4u8,
                            56u8, 234u8, 114u8, 141u8, 136u8, 127u8, 224u8, 142u8, 89u8, 150u8,
                            33u8, 31u8, 50u8, 140u8, 108u8, 124u8, 77u8, 188u8, 102u8, 230u8,
                            174u8,
                        ],
                    )
                }
                #[doc = " Similar to `ErasStakers`, this holds the preferences of validators."]
                #[doc = ""]
                #[doc = " This is keyed first by the era index to allow bulk deletion and then the stash account."]
                #[doc = ""]
                #[doc = " Is it removed after `HISTORY_DEPTH` eras."]
                pub fn eras_validator_prefs_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::ValidatorPrefs,
                    >,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ErasValidatorPrefs",
                        Vec::new(),
                        [
                            6u8, 196u8, 209u8, 138u8, 252u8, 18u8, 203u8, 86u8, 129u8, 62u8, 4u8,
                            56u8, 234u8, 114u8, 141u8, 136u8, 127u8, 224u8, 142u8, 89u8, 150u8,
                            33u8, 31u8, 50u8, 140u8, 108u8, 124u8, 77u8, 188u8, 102u8, 230u8,
                            174u8,
                        ],
                    )
                }
                #[doc = " The total validator era payout for the last `HISTORY_DEPTH` eras."]
                #[doc = ""]
                #[doc = " Eras that haven't finished yet or has been removed doesn't have reward."]
                pub fn eras_validator_reward(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ErasValidatorReward",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            87u8, 80u8, 156u8, 123u8, 107u8, 77u8, 203u8, 37u8, 231u8, 84u8, 124u8,
                            155u8, 227u8, 212u8, 212u8, 179u8, 84u8, 161u8, 223u8, 255u8, 254u8,
                            107u8, 52u8, 89u8, 98u8, 169u8, 136u8, 241u8, 104u8, 3u8, 244u8, 161u8,
                        ],
                    )
                }
                #[doc = " The total validator era payout for the last `HISTORY_DEPTH` eras."]
                #[doc = ""]
                #[doc = " Eras that haven't finished yet or has been removed doesn't have reward."]
                pub fn eras_validator_reward_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ErasValidatorReward",
                        Vec::new(),
                        [
                            87u8, 80u8, 156u8, 123u8, 107u8, 77u8, 203u8, 37u8, 231u8, 84u8, 124u8,
                            155u8, 227u8, 212u8, 212u8, 179u8, 84u8, 161u8, 223u8, 255u8, 254u8,
                            107u8, 52u8, 89u8, 98u8, 169u8, 136u8, 241u8, 104u8, 3u8, 244u8, 161u8,
                        ],
                    )
                }
                #[doc = " Rewards for the last `HISTORY_DEPTH` eras."]
                #[doc = " If reward hasn't been set or has been removed then 0 reward is returned."]
                pub fn eras_reward_points(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::EraRewardPoints<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ErasRewardPoints",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            194u8, 29u8, 20u8, 83u8, 200u8, 47u8, 158u8, 102u8, 88u8, 65u8, 24u8,
                            255u8, 120u8, 178u8, 23u8, 232u8, 15u8, 64u8, 206u8, 0u8, 170u8, 40u8,
                            18u8, 149u8, 45u8, 90u8, 179u8, 127u8, 52u8, 59u8, 37u8, 192u8,
                        ],
                    )
                }
                #[doc = " Rewards for the last `HISTORY_DEPTH` eras."]
                #[doc = " If reward hasn't been set or has been removed then 0 reward is returned."]
                pub fn eras_reward_points_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::EraRewardPoints<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                        >,
                    >,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ErasRewardPoints",
                        Vec::new(),
                        [
                            194u8, 29u8, 20u8, 83u8, 200u8, 47u8, 158u8, 102u8, 88u8, 65u8, 24u8,
                            255u8, 120u8, 178u8, 23u8, 232u8, 15u8, 64u8, 206u8, 0u8, 170u8, 40u8,
                            18u8, 149u8, 45u8, 90u8, 179u8, 127u8, 52u8, 59u8, 37u8, 192u8,
                        ],
                    )
                }
                #[doc = " The total amount staked for the last `HISTORY_DEPTH` eras."]
                #[doc = " If total hasn't been set or has been removed then 0 stake is returned."]
                pub fn eras_total_stake(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ErasTotalStake",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            224u8, 240u8, 168u8, 69u8, 148u8, 140u8, 249u8, 240u8, 4u8, 46u8, 77u8,
                            11u8, 224u8, 65u8, 26u8, 239u8, 1u8, 110u8, 53u8, 11u8, 247u8, 235u8,
                            142u8, 234u8, 22u8, 43u8, 24u8, 36u8, 37u8, 43u8, 170u8, 40u8,
                        ],
                    )
                }
                #[doc = " The total amount staked for the last `HISTORY_DEPTH` eras."]
                #[doc = " If total hasn't been set or has been removed then 0 stake is returned."]
                pub fn eras_total_stake_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ErasTotalStake",
                        Vec::new(),
                        [
                            224u8, 240u8, 168u8, 69u8, 148u8, 140u8, 249u8, 240u8, 4u8, 46u8, 77u8,
                            11u8, 224u8, 65u8, 26u8, 239u8, 1u8, 110u8, 53u8, 11u8, 247u8, 235u8,
                            142u8, 234u8, 22u8, 43u8, 24u8, 36u8, 37u8, 43u8, 170u8, 40u8,
                        ],
                    )
                }
                #[doc = " Mode of era forcing."]
                pub fn force_era(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::pallet_staking::Forcing>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ForceEra",
                        vec![],
                        [
                            221u8, 41u8, 71u8, 21u8, 28u8, 193u8, 65u8, 97u8, 103u8, 37u8, 145u8,
                            146u8, 183u8, 194u8, 57u8, 131u8, 214u8, 136u8, 68u8, 156u8, 140u8,
                            194u8, 69u8, 151u8, 115u8, 177u8, 92u8, 147u8, 29u8, 40u8, 41u8, 31u8,
                        ],
                    )
                }
                #[doc = " The percentage of the slash that is distributed to reporters."]
                #[doc = ""]
                #[doc = " The rest of the slashed value is handled by the `Slash`."]
                pub fn slash_reward_fraction(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_arithmetic::per_things::Perbill,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "SlashRewardFraction",
                        vec![],
                        [
                            167u8, 79u8, 143u8, 202u8, 199u8, 100u8, 129u8, 162u8, 23u8, 165u8,
                            106u8, 170u8, 244u8, 86u8, 144u8, 242u8, 65u8, 207u8, 115u8, 224u8,
                            231u8, 155u8, 55u8, 139u8, 101u8, 129u8, 242u8, 196u8, 130u8, 50u8,
                            3u8, 117u8,
                        ],
                    )
                }
                #[doc = " The amount of currency given to reporters of a slash event which was"]
                #[doc = " canceled by extraordinary circumstances (e.g. governance)."]
                pub fn canceled_slash_payout(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "CanceledSlashPayout",
                        vec![],
                        [
                            126u8, 218u8, 66u8, 92u8, 82u8, 124u8, 145u8, 161u8, 40u8, 176u8, 14u8,
                            211u8, 178u8, 216u8, 8u8, 156u8, 83u8, 14u8, 91u8, 15u8, 200u8, 170u8,
                            3u8, 127u8, 141u8, 139u8, 151u8, 98u8, 74u8, 96u8, 238u8, 29u8,
                        ],
                    )
                }
                #[doc = " All unapplied slashes that are queued for later."]
                pub fn unapplied_slashes(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        ::std::vec::Vec<
                            runtime_types::pallet_staking::UnappliedSlash<
                                ::subxt::ext::sp_core::crypto::AccountId32,
                                ::core::primitive::u128,
                            >,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "UnappliedSlashes",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            130u8, 4u8, 163u8, 163u8, 28u8, 85u8, 34u8, 156u8, 47u8, 125u8, 57u8,
                            0u8, 133u8, 176u8, 130u8, 2u8, 175u8, 180u8, 167u8, 203u8, 230u8, 82u8,
                            198u8, 183u8, 55u8, 82u8, 221u8, 248u8, 100u8, 173u8, 206u8, 151u8,
                        ],
                    )
                }
                #[doc = " All unapplied slashes that are queued for later."]
                pub fn unapplied_slashes_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        ::std::vec::Vec<
                            runtime_types::pallet_staking::UnappliedSlash<
                                ::subxt::ext::sp_core::crypto::AccountId32,
                                ::core::primitive::u128,
                            >,
                        >,
                    >,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "UnappliedSlashes",
                        Vec::new(),
                        [
                            130u8, 4u8, 163u8, 163u8, 28u8, 85u8, 34u8, 156u8, 47u8, 125u8, 57u8,
                            0u8, 133u8, 176u8, 130u8, 2u8, 175u8, 180u8, 167u8, 203u8, 230u8, 82u8,
                            198u8, 183u8, 55u8, 82u8, 221u8, 248u8, 100u8, 173u8, 206u8, 151u8,
                        ],
                    )
                }
                #[doc = " A mapping from still-bonded eras to the first session index of that era."]
                #[doc = ""]
                #[doc = " Must contains information for eras for the range:"]
                #[doc = " `[active_era - bounding_duration; active_era]`"]
                pub fn bonded_eras(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        ::std::vec::Vec<(::core::primitive::u32, ::core::primitive::u32)>,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "BondedEras",
                        vec![],
                        [
                            243u8, 162u8, 236u8, 198u8, 122u8, 182u8, 37u8, 55u8, 171u8, 156u8,
                            235u8, 223u8, 226u8, 129u8, 89u8, 206u8, 2u8, 155u8, 222u8, 154u8,
                            116u8, 124u8, 4u8, 119u8, 155u8, 94u8, 248u8, 30u8, 171u8, 51u8, 78u8,
                            106u8,
                        ],
                    )
                }
                #[doc = " All slashing events on validators, mapped by era to the highest slash proportion"]
                #[doc = " and slash value of the era."]
                pub fn validator_slash_in_era(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                    _1: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<(
                        runtime_types::sp_arithmetic::per_things::Perbill,
                        ::core::primitive::u128,
                    )>,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ValidatorSlashInEra",
                        vec![
                            ::subxt::storage::address::StorageMapKey::new(
                                _0.borrow(),
                                ::subxt::storage::address::StorageHasher::Twox64Concat,
                            ),
                            ::subxt::storage::address::StorageMapKey::new(
                                _1.borrow(),
                                ::subxt::storage::address::StorageHasher::Twox64Concat,
                            ),
                        ],
                        [
                            237u8, 80u8, 3u8, 237u8, 9u8, 40u8, 212u8, 15u8, 251u8, 196u8, 85u8,
                            29u8, 27u8, 151u8, 98u8, 122u8, 189u8, 147u8, 205u8, 40u8, 202u8,
                            194u8, 158u8, 96u8, 138u8, 16u8, 116u8, 71u8, 140u8, 163u8, 121u8,
                            197u8,
                        ],
                    )
                }
                #[doc = " All slashing events on validators, mapped by era to the highest slash proportion"]
                #[doc = " and slash value of the era."]
                pub fn validator_slash_in_era_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<(
                        runtime_types::sp_arithmetic::per_things::Perbill,
                        ::core::primitive::u128,
                    )>,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ValidatorSlashInEra",
                        Vec::new(),
                        [
                            237u8, 80u8, 3u8, 237u8, 9u8, 40u8, 212u8, 15u8, 251u8, 196u8, 85u8,
                            29u8, 27u8, 151u8, 98u8, 122u8, 189u8, 147u8, 205u8, 40u8, 202u8,
                            194u8, 158u8, 96u8, 138u8, 16u8, 116u8, 71u8, 140u8, 163u8, 121u8,
                            197u8,
                        ],
                    )
                }
                #[doc = " All slashing events on nominators, mapped by era to the highest slash value of the era."]
                pub fn nominator_slash_in_era(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                    _1: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "NominatorSlashInEra",
                        vec![
                            ::subxt::storage::address::StorageMapKey::new(
                                _0.borrow(),
                                ::subxt::storage::address::StorageHasher::Twox64Concat,
                            ),
                            ::subxt::storage::address::StorageMapKey::new(
                                _1.borrow(),
                                ::subxt::storage::address::StorageHasher::Twox64Concat,
                            ),
                        ],
                        [
                            249u8, 85u8, 170u8, 41u8, 179u8, 194u8, 180u8, 12u8, 53u8, 101u8, 80u8,
                            96u8, 166u8, 71u8, 239u8, 23u8, 153u8, 19u8, 152u8, 38u8, 138u8, 136u8,
                            221u8, 200u8, 18u8, 165u8, 26u8, 228u8, 195u8, 199u8, 62u8, 4u8,
                        ],
                    )
                }
                #[doc = " All slashing events on nominators, mapped by era to the highest slash value of the era."]
                pub fn nominator_slash_in_era_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "NominatorSlashInEra",
                        Vec::new(),
                        [
                            249u8, 85u8, 170u8, 41u8, 179u8, 194u8, 180u8, 12u8, 53u8, 101u8, 80u8,
                            96u8, 166u8, 71u8, 239u8, 23u8, 153u8, 19u8, 152u8, 38u8, 138u8, 136u8,
                            221u8, 200u8, 18u8, 165u8, 26u8, 228u8, 195u8, 199u8, 62u8, 4u8,
                        ],
                    )
                }
                #[doc = " Slashing spans for stash accounts."]
                pub fn slashing_spans(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::slashing::SlashingSpans,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "SlashingSpans",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            106u8, 115u8, 118u8, 52u8, 89u8, 77u8, 246u8, 5u8, 255u8, 204u8, 44u8,
                            5u8, 66u8, 36u8, 227u8, 252u8, 86u8, 159u8, 186u8, 152u8, 196u8, 21u8,
                            74u8, 201u8, 133u8, 93u8, 142u8, 191u8, 20u8, 27u8, 218u8, 157u8,
                        ],
                    )
                }
                #[doc = " Slashing spans for stash accounts."]
                pub fn slashing_spans_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::slashing::SlashingSpans,
                    >,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "SlashingSpans",
                        Vec::new(),
                        [
                            106u8, 115u8, 118u8, 52u8, 89u8, 77u8, 246u8, 5u8, 255u8, 204u8, 44u8,
                            5u8, 66u8, 36u8, 227u8, 252u8, 86u8, 159u8, 186u8, 152u8, 196u8, 21u8,
                            74u8, 201u8, 133u8, 93u8, 142u8, 191u8, 20u8, 27u8, 218u8, 157u8,
                        ],
                    )
                }
                #[doc = " Records information about the maximum slash of a stash within a slashing span,"]
                #[doc = " as well as how much reward has been paid out."]
                pub fn span_slash(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                    _1: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::slashing::SpanRecord<
                            ::core::primitive::u128,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "SpanSlash",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            &(_0.borrow(), _1.borrow()),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            160u8, 63u8, 115u8, 190u8, 233u8, 148u8, 75u8, 3u8, 11u8, 59u8, 184u8,
                            220u8, 205u8, 64u8, 28u8, 190u8, 116u8, 210u8, 225u8, 230u8, 224u8,
                            163u8, 103u8, 157u8, 100u8, 29u8, 86u8, 167u8, 84u8, 217u8, 109u8,
                            200u8,
                        ],
                    )
                }
                #[doc = " Records information about the maximum slash of a stash within a slashing span,"]
                #[doc = " as well as how much reward has been paid out."]
                pub fn span_slash_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_staking::slashing::SpanRecord<
                            ::core::primitive::u128,
                        >,
                    >,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "SpanSlash",
                        Vec::new(),
                        [
                            160u8, 63u8, 115u8, 190u8, 233u8, 148u8, 75u8, 3u8, 11u8, 59u8, 184u8,
                            220u8, 205u8, 64u8, 28u8, 190u8, 116u8, 210u8, 225u8, 230u8, 224u8,
                            163u8, 103u8, 157u8, 100u8, 29u8, 86u8, 167u8, 84u8, 217u8, 109u8,
                            200u8,
                        ],
                    )
                }
                #[doc = " The last planned session scheduled by the session pallet."]
                #[doc = ""]
                #[doc = " This is basically in sync with the call to [`pallet_session::SessionManager::new_session`]."]
                pub fn current_planned_session(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "CurrentPlannedSession",
                        vec![],
                        [
                            38u8, 22u8, 56u8, 250u8, 17u8, 154u8, 99u8, 37u8, 155u8, 253u8, 100u8,
                            117u8, 5u8, 239u8, 31u8, 190u8, 53u8, 241u8, 11u8, 185u8, 163u8, 227u8,
                            10u8, 77u8, 210u8, 64u8, 156u8, 218u8, 105u8, 16u8, 1u8, 57u8,
                        ],
                    )
                }
                #[doc = " Indices of validators that have offended in the active era and whether they are currently"]
                #[doc = " disabled."]
                #[doc = ""]
                #[doc = " This value should be a superset of disabled validators since not all offences lead to the"]
                #[doc = " validator being disabled (if there was no slash). This is needed to track the percentage of"]
                #[doc = " validators that have offended in the current era, ensuring a new era is forced if"]
                #[doc = " `OffendingValidatorsThreshold` is reached. The vec is always kept sorted so that we can find"]
                #[doc = " whether a given validator has previously offended using binary search. It gets cleared when"]
                #[doc = " the era ends."]
                pub fn offending_validators(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        ::std::vec::Vec<(::core::primitive::u32, ::core::primitive::bool)>,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "OffendingValidators",
                        vec![],
                        [
                            94u8, 254u8, 0u8, 50u8, 76u8, 232u8, 51u8, 153u8, 118u8, 14u8, 70u8,
                            101u8, 112u8, 215u8, 173u8, 82u8, 182u8, 104u8, 167u8, 103u8, 187u8,
                            168u8, 86u8, 16u8, 51u8, 235u8, 51u8, 119u8, 38u8, 154u8, 42u8, 113u8,
                        ],
                    )
                }
                #[doc = " The threshold for when users can start calling `chill_other` for other validators /"]
                #[doc = " nominators. The threshold is compared to the actual number of validators / nominators"]
                #[doc = " (`CountFor*`) in the system compared to the configured max (`Max*Count`)."]
                pub fn chill_threshold(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_arithmetic::per_things::Percent,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Staking",
                        "ChillThreshold",
                        vec![],
                        [
                            174u8, 165u8, 249u8, 105u8, 24u8, 151u8, 115u8, 166u8, 199u8, 251u8,
                            28u8, 5u8, 50u8, 95u8, 144u8, 110u8, 220u8, 76u8, 14u8, 23u8, 179u8,
                            41u8, 11u8, 248u8, 28u8, 154u8, 159u8, 255u8, 156u8, 109u8, 98u8, 92u8,
                        ],
                    )
                }
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                #[doc = " Maximum number of nominations per nominator."]
                pub fn max_nominations(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Staking",
                        "MaxNominations",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                #[doc = " Number of eras to keep in history."]
                #[doc = ""]
                #[doc = " Following information is kept for eras in `[current_era -"]
                #[doc = " HistoryDepth, current_era]`: `ErasStakers`, `ErasStakersClipped`,"]
                #[doc = " `ErasValidatorPrefs`, `ErasValidatorReward`, `ErasRewardPoints`,"]
                #[doc = " `ErasTotalStake`, `ErasStartSessionIndex`,"]
                #[doc = " `StakingLedger.claimed_rewards`."]
                #[doc = ""]
                #[doc = " Must be more than the number of eras delayed by session."]
                #[doc = " I.e. active era must always be in history. I.e. `active_era >"]
                #[doc = " current_era - history_depth` must be guaranteed."]
                #[doc = ""]
                #[doc = " If migrating an existing pallet from storage value to config value,"]
                #[doc = " this should be set to same value or greater as in storage."]
                #[doc = ""]
                #[doc = " Note: `HistoryDepth` is used as the upper bound for the `BoundedVec`"]
                #[doc = " item `StakingLedger.claimed_rewards`. Setting this value lower than"]
                #[doc = " the existing value can lead to inconsistencies in the"]
                #[doc = " `StakingLedger` and will need to be handled properly in a migration."]
                #[doc = " The test `reducing_history_depth_abrupt` shows this effect."]
                pub fn history_depth(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Staking",
                        "HistoryDepth",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                #[doc = " Number of sessions per era."]
                pub fn sessions_per_era(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Staking",
                        "SessionsPerEra",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                #[doc = " Number of eras that staked funds must remain bonded for."]
                pub fn bonding_duration(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Staking",
                        "BondingDuration",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                #[doc = " Number of eras that slashes are deferred by, after computation."]
                #[doc = ""]
                #[doc = " This should be less than the bonding duration. Set to 0 if slashes"]
                #[doc = " should be applied immediately, without opportunity for intervention."]
                pub fn slash_defer_duration(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Staking",
                        "SlashDeferDuration",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                #[doc = " The maximum number of nominators rewarded for each validator."]
                #[doc = ""]
                #[doc = " For each validator only the `$MaxNominatorRewardedPerValidator` biggest stakers can"]
                #[doc = " claim their reward. This used to limit the i/o cost for the nominator payout."]
                pub fn max_nominator_rewarded_per_validator(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Staking",
                        "MaxNominatorRewardedPerValidator",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                #[doc = " The maximum number of `unlocking` chunks a [`StakingLedger`] can"]
                #[doc = " have. Effectively determines how many unique eras a staker may be"]
                #[doc = " unbonding in."]
                #[doc = ""]
                #[doc = " Note: `MaxUnlockingChunks` is used as the upper bound for the"]
                #[doc = " `BoundedVec` item `StakingLedger.unlocking`. Setting this value"]
                #[doc = " lower than the existing value can lead to inconsistencies in the"]
                #[doc = " `StakingLedger` and will need to be handled properly in a runtime"]
                #[doc = " migration. The test `reducing_max_unlocking_chunks_abrupt` shows"]
                #[doc = " this effect."]
                pub fn max_unlocking_chunks(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Staking",
                        "MaxUnlockingChunks",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod history {
        use super::{root_mod, runtime_types};
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                #[doc = " Mapping from historical session indices to session-data root hash and validator count."]
                pub fn historical_sessions(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<(
                        ::subxt::ext::sp_core::H256,
                        ::core::primitive::u32,
                    )>,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "History",
                        "HistoricalSessions",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            94u8, 72u8, 245u8, 151u8, 214u8, 10u8, 12u8, 113u8, 13u8, 141u8, 176u8,
                            178u8, 115u8, 238u8, 224u8, 181u8, 18u8, 5u8, 71u8, 65u8, 189u8, 148u8,
                            161u8, 106u8, 24u8, 211u8, 72u8, 66u8, 221u8, 244u8, 117u8, 184u8,
                        ],
                    )
                }
                #[doc = " Mapping from historical session indices to session-data root hash and validator count."]
                pub fn historical_sessions_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<(
                        ::subxt::ext::sp_core::H256,
                        ::core::primitive::u32,
                    )>,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "History",
                        "HistoricalSessions",
                        Vec::new(),
                        [
                            94u8, 72u8, 245u8, 151u8, 214u8, 10u8, 12u8, 113u8, 13u8, 141u8, 176u8,
                            178u8, 115u8, 238u8, 224u8, 181u8, 18u8, 5u8, 71u8, 65u8, 189u8, 148u8,
                            161u8, 106u8, 24u8, 211u8, 72u8, 66u8, 221u8, 244u8, 117u8, 184u8,
                        ],
                    )
                }
                #[doc = " The range of historical sessions we store. [first, last)"]
                pub fn stored_range(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<(
                        ::core::primitive::u32,
                        ::core::primitive::u32,
                    )>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "History",
                        "StoredRange",
                        vec![],
                        [
                            89u8, 239u8, 197u8, 93u8, 135u8, 62u8, 142u8, 237u8, 64u8, 200u8,
                            164u8, 4u8, 130u8, 233u8, 16u8, 238u8, 166u8, 206u8, 71u8, 42u8, 171u8,
                            84u8, 8u8, 245u8, 183u8, 216u8, 212u8, 16u8, 190u8, 3u8, 167u8, 189u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod session {
        use super::{root_mod, runtime_types};
        #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetKeys {
                pub keys: runtime_types::aleph_runtime::SessionKeys,
                pub proof: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct PurgeKeys;
            pub struct TransactionApi;
            impl TransactionApi {
                #[doc = "Sets the session key(s) of the function caller to `keys`."]
                #[doc = "Allows an account to set its session key prior to becoming a validator."]
                #[doc = "This doesn't take effect until the next session."]
                #[doc = ""]
                #[doc = "The dispatch origin of this function must be signed."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- Complexity: `O(1)`. Actual cost depends on the number of length of"]
                #[doc = "  `T::Keys::key_ids()` which is fixed."]
                #[doc = "- DbReads: `origin account`, `T::ValidatorIdOf`, `NextKeys`"]
                #[doc = "- DbWrites: `origin account`, `NextKeys`"]
                #[doc = "- DbReads per key id: `KeyOwner`"]
                #[doc = "- DbWrites per key id: `KeyOwner`"]
                #[doc = "# </weight>"]
                pub fn set_keys(
                    &self,
                    keys: runtime_types::aleph_runtime::SessionKeys,
                    proof: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::StaticTxPayload<SetKeys> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Session",
                        "set_keys",
                        SetKeys { keys, proof },
                        [
                            104u8, 12u8, 102u8, 88u8, 135u8, 226u8, 42u8, 162u8, 217u8, 56u8,
                            227u8, 24u8, 190u8, 82u8, 1u8, 41u8, 49u8, 50u8, 146u8, 96u8, 21u8,
                            56u8, 131u8, 32u8, 244u8, 189u8, 95u8, 22u8, 219u8, 106u8, 236u8,
                            206u8,
                        ],
                    )
                }
                #[doc = "Removes any session key(s) of the function caller."]
                #[doc = ""]
                #[doc = "This doesn't take effect until the next session."]
                #[doc = ""]
                #[doc = "The dispatch origin of this function must be Signed and the account must be either be"]
                #[doc = "convertible to a validator ID using the chain's typical addressing system (this usually"]
                #[doc = "means being a controller account) or directly convertible into a validator ID (which"]
                #[doc = "usually means being a stash account)."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- Complexity: `O(1)` in number of key types. Actual cost depends on the number of length"]
                #[doc = "  of `T::Keys::key_ids()` which is fixed."]
                #[doc = "- DbReads: `T::ValidatorIdOf`, `NextKeys`, `origin account`"]
                #[doc = "- DbWrites: `NextKeys`, `origin account`"]
                #[doc = "- DbWrites per key id: `KeyOwner`"]
                #[doc = "# </weight>"]
                pub fn purge_keys(&self) -> ::subxt::tx::StaticTxPayload<PurgeKeys> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Session",
                        "purge_keys",
                        PurgeKeys {},
                        [
                            200u8, 255u8, 4u8, 213u8, 188u8, 92u8, 99u8, 116u8, 163u8, 152u8, 29u8,
                            35u8, 133u8, 119u8, 246u8, 44u8, 91u8, 31u8, 145u8, 23u8, 213u8, 64u8,
                            71u8, 242u8, 207u8, 239u8, 231u8, 37u8, 61u8, 63u8, 190u8, 35u8,
                        ],
                    )
                }
            }
        }
        #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
        pub type Event = runtime_types::pallet_session::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: CompactAs,
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "New session has happened. Note that the argument is the session index, not the"]
            #[doc = "block number as the type might suggest."]
            pub struct NewSession {
                pub session_index: ::core::primitive::u32,
            }
            impl ::subxt::events::StaticEvent for NewSession {
                const PALLET: &'static str = "Session";
                const EVENT: &'static str = "NewSession";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                #[doc = " The current set of validators."]
                pub fn validators(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Session",
                        "Validators",
                        vec![],
                        [
                            144u8, 235u8, 200u8, 43u8, 151u8, 57u8, 147u8, 172u8, 201u8, 202u8,
                            242u8, 96u8, 57u8, 76u8, 124u8, 77u8, 42u8, 113u8, 218u8, 220u8, 230u8,
                            32u8, 151u8, 152u8, 172u8, 106u8, 60u8, 227u8, 122u8, 118u8, 137u8,
                            68u8,
                        ],
                    )
                }
                #[doc = " Current index of the session."]
                pub fn current_index(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Session",
                        "CurrentIndex",
                        vec![],
                        [
                            148u8, 179u8, 159u8, 15u8, 197u8, 95u8, 214u8, 30u8, 209u8, 251u8,
                            183u8, 231u8, 91u8, 25u8, 181u8, 191u8, 143u8, 252u8, 227u8, 80u8,
                            159u8, 66u8, 194u8, 67u8, 113u8, 74u8, 111u8, 91u8, 218u8, 187u8,
                            130u8, 40u8,
                        ],
                    )
                }
                #[doc = " True if the underlying economic identities or weighting behind the validators"]
                #[doc = " has changed in the queued validator set."]
                pub fn queued_changed(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::bool>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Session",
                        "QueuedChanged",
                        vec![],
                        [
                            105u8, 140u8, 235u8, 218u8, 96u8, 100u8, 252u8, 10u8, 58u8, 221u8,
                            244u8, 251u8, 67u8, 91u8, 80u8, 202u8, 152u8, 42u8, 50u8, 113u8, 200u8,
                            247u8, 59u8, 213u8, 77u8, 195u8, 1u8, 150u8, 220u8, 18u8, 245u8, 46u8,
                        ],
                    )
                }
                #[doc = " The queued keys for the next session. When the next session begins, these keys"]
                #[doc = " will be used to determine the validator's session keys."]
                pub fn queued_keys(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        ::std::vec::Vec<(
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            runtime_types::aleph_runtime::SessionKeys,
                        )>,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Session",
                        "QueuedKeys",
                        vec![],
                        [
                            225u8, 184u8, 241u8, 120u8, 191u8, 179u8, 152u8, 85u8, 19u8, 139u8,
                            177u8, 231u8, 102u8, 210u8, 125u8, 68u8, 196u8, 242u8, 40u8, 39u8,
                            70u8, 16u8, 6u8, 167u8, 81u8, 190u8, 61u8, 91u8, 246u8, 206u8, 249u8,
                            78u8,
                        ],
                    )
                }
                #[doc = " Indices of disabled validators."]
                #[doc = ""]
                #[doc = " The vec is always kept sorted so that we can find whether a given validator is"]
                #[doc = " disabled using binary search. It gets cleared when `on_session_ending` returns"]
                #[doc = " a new set of identities."]
                pub fn disabled_validators(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::std::vec::Vec<::core::primitive::u32>>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Session",
                        "DisabledValidators",
                        vec![],
                        [
                            135u8, 22u8, 22u8, 97u8, 82u8, 217u8, 144u8, 141u8, 121u8, 240u8,
                            189u8, 16u8, 176u8, 88u8, 177u8, 31u8, 20u8, 242u8, 73u8, 104u8, 11u8,
                            110u8, 214u8, 34u8, 52u8, 217u8, 106u8, 33u8, 174u8, 174u8, 198u8,
                            84u8,
                        ],
                    )
                }
                #[doc = " The next session keys for a validator."]
                pub fn next_keys(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::aleph_runtime::SessionKeys>,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Session",
                        "NextKeys",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            18u8, 209u8, 31u8, 20u8, 131u8, 9u8, 97u8, 157u8, 63u8, 9u8, 233u8,
                            216u8, 40u8, 240u8, 66u8, 111u8, 60u8, 87u8, 83u8, 178u8, 17u8, 105u8,
                            214u8, 169u8, 171u8, 220u8, 7u8, 121u8, 35u8, 229u8, 253u8, 40u8,
                        ],
                    )
                }
                #[doc = " The next session keys for a validator."]
                pub fn next_keys_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::aleph_runtime::SessionKeys>,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Session",
                        "NextKeys",
                        Vec::new(),
                        [
                            18u8, 209u8, 31u8, 20u8, 131u8, 9u8, 97u8, 157u8, 63u8, 9u8, 233u8,
                            216u8, 40u8, 240u8, 66u8, 111u8, 60u8, 87u8, 83u8, 178u8, 17u8, 105u8,
                            214u8, 169u8, 171u8, 220u8, 7u8, 121u8, 35u8, 229u8, 253u8, 40u8,
                        ],
                    )
                }
                #[doc = " The owner of a key. The key is the `KeyTypeId` + the encoded key."]
                pub fn key_owner(
                    &self,
                    _0: impl ::std::borrow::Borrow<runtime_types::sp_core::crypto::KeyTypeId>,
                    _1: impl ::std::borrow::Borrow<[::core::primitive::u8]>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::subxt::ext::sp_core::crypto::AccountId32>,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Session",
                        "KeyOwner",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            &(_0.borrow(), _1.borrow()),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            4u8, 91u8, 25u8, 84u8, 250u8, 201u8, 174u8, 129u8, 201u8, 58u8, 197u8,
                            199u8, 137u8, 240u8, 118u8, 33u8, 99u8, 2u8, 195u8, 57u8, 53u8, 172u8,
                            0u8, 148u8, 203u8, 144u8, 149u8, 64u8, 135u8, 254u8, 242u8, 215u8,
                        ],
                    )
                }
                #[doc = " The owner of a key. The key is the `KeyTypeId` + the encoded key."]
                pub fn key_owner_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::subxt::ext::sp_core::crypto::AccountId32>,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Session",
                        "KeyOwner",
                        Vec::new(),
                        [
                            4u8, 91u8, 25u8, 84u8, 250u8, 201u8, 174u8, 129u8, 201u8, 58u8, 197u8,
                            199u8, 137u8, 240u8, 118u8, 33u8, 99u8, 2u8, 195u8, 57u8, 53u8, 172u8,
                            0u8, 148u8, 203u8, 144u8, 149u8, 64u8, 135u8, 254u8, 242u8, 215u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod aleph {
        use super::{root_mod, runtime_types};
        #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetEmergencyFinalizer {
                pub emergency_finalizer: runtime_types::primitives::app::Public,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ScheduleFinalityVersionChange {
                pub version_incoming: ::core::primitive::u32,
                pub session: ::core::primitive::u32,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                #[doc = "Sets the emergency finalization key. If called in session `N` the key can be used to"]
                #[doc = "finalize blocks from session `N+2` onwards, until it gets overridden."]
                pub fn set_emergency_finalizer(
                    &self,
                    emergency_finalizer: runtime_types::primitives::app::Public,
                ) -> ::subxt::tx::StaticTxPayload<SetEmergencyFinalizer> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Aleph",
                        "set_emergency_finalizer",
                        SetEmergencyFinalizer {
                            emergency_finalizer,
                        },
                        [
                            13u8, 25u8, 102u8, 60u8, 64u8, 83u8, 158u8, 113u8, 177u8, 194u8, 79u8,
                            66u8, 171u8, 98u8, 169u8, 52u8, 69u8, 194u8, 54u8, 131u8, 190u8, 83u8,
                            21u8, 64u8, 119u8, 90u8, 53u8, 125u8, 52u8, 155u8, 222u8, 76u8,
                        ],
                    )
                }
                #[doc = "Schedules a finality version change for a future session. If such a scheduled future"]
                #[doc = "version is already set, it is replaced with the provided one."]
                #[doc = "Any rescheduling of a future version change needs to occur at least 2 sessions in"]
                #[doc = "advance of the provided session of the version change."]
                #[doc = "In order to cancel a scheduled version change, a new version change should be scheduled"]
                #[doc = "with the same version as the current one."]
                pub fn schedule_finality_version_change(
                    &self,
                    version_incoming: ::core::primitive::u32,
                    session: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<ScheduleFinalityVersionChange> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Aleph",
                        "schedule_finality_version_change",
                        ScheduleFinalityVersionChange {
                            version_incoming,
                            session,
                        },
                        [
                            27u8, 162u8, 238u8, 141u8, 132u8, 87u8, 69u8, 115u8, 243u8, 197u8,
                            38u8, 37u8, 243u8, 86u8, 45u8, 137u8, 73u8, 181u8, 108u8, 200u8, 168u8,
                            141u8, 130u8, 244u8, 85u8, 128u8, 145u8, 34u8, 233u8, 87u8, 38u8,
                            198u8,
                        ],
                    )
                }
            }
        }
        #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
        pub type Event = runtime_types::pallet_aleph::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ChangeEmergencyFinalizer(pub runtime_types::primitives::app::Public);
            impl ::subxt::events::StaticEvent for ChangeEmergencyFinalizer {
                const PALLET: &'static str = "Aleph";
                const EVENT: &'static str = "ChangeEmergencyFinalizer";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ScheduleFinalityVersionChange(pub runtime_types::primitives::VersionChange);
            impl ::subxt::events::StaticEvent for ScheduleFinalityVersionChange {
                const PALLET: &'static str = "Aleph";
                const EVENT: &'static str = "ScheduleFinalityVersionChange";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct FinalityVersionChange(pub runtime_types::primitives::VersionChange);
            impl ::subxt::events::StaticEvent for FinalityVersionChange {
                const PALLET: &'static str = "Aleph";
                const EVENT: &'static str = "FinalityVersionChange";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                pub fn authorities(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        ::std::vec::Vec<runtime_types::primitives::app::Public>,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Aleph",
                        "Authorities",
                        vec![],
                        [
                            5u8, 88u8, 222u8, 150u8, 141u8, 89u8, 69u8, 47u8, 152u8, 80u8, 80u8,
                            1u8, 20u8, 132u8, 5u8, 152u8, 175u8, 14u8, 99u8, 198u8, 102u8, 229u8,
                            159u8, 198u8, 138u8, 149u8, 68u8, 195u8, 243u8, 50u8, 249u8, 170u8,
                        ],
                    )
                }
                pub fn next_authorities(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        ::std::vec::Vec<runtime_types::primitives::app::Public>,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Aleph",
                        "NextAuthorities",
                        vec![],
                        [
                            38u8, 130u8, 45u8, 205u8, 118u8, 26u8, 81u8, 123u8, 200u8, 81u8, 73u8,
                            207u8, 22u8, 31u8, 63u8, 38u8, 121u8, 42u8, 250u8, 65u8, 75u8, 79u8,
                            16u8, 10u8, 137u8, 46u8, 173u8, 104u8, 109u8, 134u8, 140u8, 3u8,
                        ],
                    )
                }
                pub fn emergency_finalizer(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::primitives::app::Public>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Aleph",
                        "EmergencyFinalizer",
                        vec![],
                        [
                            254u8, 68u8, 214u8, 192u8, 214u8, 1u8, 48u8, 167u8, 1u8, 55u8, 148u8,
                            124u8, 72u8, 123u8, 148u8, 50u8, 131u8, 17u8, 48u8, 14u8, 48u8, 92u8,
                            3u8, 56u8, 60u8, 224u8, 97u8, 60u8, 208u8, 53u8, 164u8, 88u8,
                        ],
                    )
                }
                pub fn queued_emergency_finalizer(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::primitives::app::Public>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Aleph",
                        "QueuedEmergencyFinalizer",
                        vec![],
                        [
                            156u8, 104u8, 166u8, 24u8, 170u8, 246u8, 39u8, 246u8, 130u8, 169u8,
                            222u8, 196u8, 137u8, 216u8, 190u8, 64u8, 28u8, 50u8, 6u8, 194u8, 164u8,
                            91u8, 85u8, 78u8, 212u8, 61u8, 126u8, 242u8, 207u8, 76u8, 227u8, 115u8,
                        ],
                    )
                }
                pub fn next_emergency_finalizer(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::primitives::app::Public>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Aleph",
                        "NextEmergencyFinalizer",
                        vec![],
                        [
                            84u8, 79u8, 27u8, 15u8, 74u8, 189u8, 24u8, 17u8, 157u8, 91u8, 245u8,
                            30u8, 129u8, 11u8, 226u8, 87u8, 50u8, 182u8, 60u8, 73u8, 214u8, 46u8,
                            132u8, 0u8, 53u8, 14u8, 228u8, 115u8, 240u8, 64u8, 158u8, 165u8,
                        ],
                    )
                }
                #[doc = " Current finality version."]
                pub fn finality_version(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Aleph",
                        "FinalityVersion",
                        vec![],
                        [
                            134u8, 19u8, 94u8, 247u8, 125u8, 18u8, 148u8, 160u8, 167u8, 235u8,
                            174u8, 4u8, 107u8, 69u8, 55u8, 187u8, 249u8, 13u8, 129u8, 99u8, 116u8,
                            158u8, 38u8, 29u8, 239u8, 112u8, 150u8, 92u8, 151u8, 197u8, 223u8,
                            30u8,
                        ],
                    )
                }
                #[doc = " Scheduled finality version change."]
                pub fn finality_scheduled_version_change(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::primitives::VersionChange>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Aleph",
                        "FinalityScheduledVersionChange",
                        vec![],
                        [
                            195u8, 203u8, 203u8, 240u8, 214u8, 227u8, 177u8, 99u8, 82u8, 86u8,
                            201u8, 237u8, 47u8, 32u8, 111u8, 219u8, 184u8, 107u8, 211u8, 83u8,
                            25u8, 59u8, 170u8, 29u8, 24u8, 149u8, 85u8, 63u8, 37u8, 203u8, 129u8,
                            97u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod elections {
        use super::{root_mod, runtime_types};
        #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ChangeValidators {
                pub reserved_validators: ::core::option::Option<
                    ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                >,
                pub non_reserved_validators: ::core::option::Option<
                    ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                >,
                pub committee_size:
                    ::core::option::Option<runtime_types::primitives::CommitteeSeats>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetBanConfig {
                pub minimal_expected_performance: ::core::option::Option<::core::primitive::u8>,
                pub underperformed_session_count_threshold:
                    ::core::option::Option<::core::primitive::u32>,
                pub clean_session_counter_delay: ::core::option::Option<::core::primitive::u32>,
                pub ban_period: ::core::option::Option<::core::primitive::u32>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct BanFromCommittee {
                pub banned: ::subxt::ext::sp_core::crypto::AccountId32,
                pub ban_reason: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct CancelBan {
                pub banned: ::subxt::ext::sp_core::crypto::AccountId32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetElectionsOpenness {
                pub openness: runtime_types::primitives::ElectionOpenness,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                pub fn change_validators(
                    &self,
                    reserved_validators: ::core::option::Option<
                        ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                    >,
                    non_reserved_validators: ::core::option::Option<
                        ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                    >,
                    committee_size: ::core::option::Option<
                        runtime_types::primitives::CommitteeSeats,
                    >,
                ) -> ::subxt::tx::StaticTxPayload<ChangeValidators> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Elections",
                        "change_validators",
                        ChangeValidators {
                            reserved_validators,
                            non_reserved_validators,
                            committee_size,
                        },
                        [
                            88u8, 2u8, 255u8, 219u8, 50u8, 103u8, 169u8, 150u8, 249u8, 161u8, 57u8,
                            39u8, 6u8, 241u8, 94u8, 139u8, 206u8, 236u8, 160u8, 92u8, 163u8, 170u8,
                            222u8, 99u8, 50u8, 91u8, 194u8, 192u8, 99u8, 123u8, 41u8, 136u8,
                        ],
                    )
                }
                #[doc = "Sets ban config, it has an immediate effect"]
                pub fn set_ban_config(
                    &self,
                    minimal_expected_performance: ::core::option::Option<::core::primitive::u8>,
                    underperformed_session_count_threshold: ::core::option::Option<
                        ::core::primitive::u32,
                    >,
                    clean_session_counter_delay: ::core::option::Option<::core::primitive::u32>,
                    ban_period: ::core::option::Option<::core::primitive::u32>,
                ) -> ::subxt::tx::StaticTxPayload<SetBanConfig> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Elections",
                        "set_ban_config",
                        SetBanConfig {
                            minimal_expected_performance,
                            underperformed_session_count_threshold,
                            clean_session_counter_delay,
                            ban_period,
                        },
                        [
                            228u8, 199u8, 170u8, 155u8, 208u8, 190u8, 211u8, 218u8, 105u8, 213u8,
                            240u8, 152u8, 92u8, 19u8, 164u8, 28u8, 215u8, 145u8, 47u8, 248u8,
                            219u8, 75u8, 234u8, 78u8, 29u8, 189u8, 35u8, 106u8, 165u8, 76u8, 27u8,
                            50u8,
                        ],
                    )
                }
                #[doc = "Schedule a non-reserved node to be banned out from the committee at the end of the era"]
                pub fn ban_from_committee(
                    &self,
                    banned: ::subxt::ext::sp_core::crypto::AccountId32,
                    ban_reason: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::StaticTxPayload<BanFromCommittee> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Elections",
                        "ban_from_committee",
                        BanFromCommittee { banned, ban_reason },
                        [
                            60u8, 254u8, 80u8, 201u8, 64u8, 189u8, 255u8, 111u8, 14u8, 9u8, 68u8,
                            177u8, 196u8, 107u8, 10u8, 177u8, 78u8, 134u8, 98u8, 21u8, 179u8, 9u8,
                            111u8, 185u8, 155u8, 39u8, 148u8, 88u8, 239u8, 16u8, 24u8, 171u8,
                        ],
                    )
                }
                #[doc = "Schedule a non-reserved node to be banned out from the committee at the end of the era"]
                pub fn cancel_ban(
                    &self,
                    banned: ::subxt::ext::sp_core::crypto::AccountId32,
                ) -> ::subxt::tx::StaticTxPayload<CancelBan> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Elections",
                        "cancel_ban",
                        CancelBan { banned },
                        [
                            103u8, 192u8, 40u8, 246u8, 206u8, 52u8, 222u8, 51u8, 39u8, 247u8,
                            220u8, 175u8, 232u8, 31u8, 168u8, 99u8, 206u8, 45u8, 191u8, 161u8,
                            107u8, 12u8, 112u8, 54u8, 163u8, 170u8, 221u8, 220u8, 122u8, 177u8,
                            178u8, 246u8,
                        ],
                    )
                }
                #[doc = "Set openness of the elections"]
                pub fn set_elections_openness(
                    &self,
                    openness: runtime_types::primitives::ElectionOpenness,
                ) -> ::subxt::tx::StaticTxPayload<SetElectionsOpenness> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Elections",
                        "set_elections_openness",
                        SetElectionsOpenness { openness },
                        [
                            207u8, 20u8, 183u8, 25u8, 206u8, 225u8, 242u8, 167u8, 164u8, 54u8,
                            111u8, 134u8, 139u8, 4u8, 7u8, 89u8, 119u8, 165u8, 53u8, 3u8, 96u8,
                            107u8, 188u8, 196u8, 113u8, 35u8, 128u8, 240u8, 222u8, 23u8, 221u8,
                            105u8,
                        ],
                    )
                }
            }
        }
        #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
        pub type Event = runtime_types::pallet_elections::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Committee for the next era has changed"]
            pub struct ChangeValidators(
                pub ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                pub ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                pub runtime_types::primitives::CommitteeSeats,
            );
            impl ::subxt::events::StaticEvent for ChangeValidators {
                const PALLET: &'static str = "Elections";
                const EVENT: &'static str = "ChangeValidators";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Ban thresholds for the next era has changed"]
            pub struct SetBanConfig(pub runtime_types::primitives::BanConfig);
            impl ::subxt::events::StaticEvent for SetBanConfig {
                const PALLET: &'static str = "Elections";
                const EVENT: &'static str = "SetBanConfig";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Validators have been banned from the committee"]
            pub struct BanValidators(
                pub  ::std::vec::Vec<(
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    runtime_types::primitives::BanInfo,
                )>,
            );
            impl ::subxt::events::StaticEvent for BanValidators {
                const PALLET: &'static str = "Elections";
                const EVENT: &'static str = "BanValidators";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                #[doc = " Desirable size of a committee, see [`CommitteeSeats`]."]
                pub fn committee_size(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::primitives::CommitteeSeats>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Elections",
                        "CommitteeSize",
                        vec![],
                        [
                            138u8, 114u8, 93u8, 183u8, 35u8, 215u8, 48u8, 195u8, 127u8, 157u8,
                            38u8, 169u8, 255u8, 246u8, 178u8, 219u8, 221u8, 247u8, 35u8, 45u8,
                            94u8, 195u8, 84u8, 36u8, 30u8, 252u8, 145u8, 90u8, 67u8, 254u8, 39u8,
                            199u8,
                        ],
                    )
                }
                #[doc = " Desired size of a committee in effect from a new era."]
                pub fn next_era_committee_size(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::primitives::CommitteeSeats>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Elections",
                        "NextEraCommitteeSize",
                        vec![],
                        [
                            1u8, 114u8, 197u8, 86u8, 178u8, 92u8, 167u8, 99u8, 96u8, 98u8, 65u8,
                            149u8, 222u8, 39u8, 119u8, 24u8, 251u8, 65u8, 171u8, 126u8, 100u8,
                            137u8, 50u8, 72u8, 108u8, 47u8, 95u8, 63u8, 202u8, 64u8, 120u8, 120u8,
                        ],
                    )
                }
                #[doc = " Next era's list of reserved validators."]
                pub fn next_era_reserved_validators(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Elections",
                        "NextEraReservedValidators",
                        vec![],
                        [
                            6u8, 123u8, 0u8, 238u8, 248u8, 8u8, 50u8, 48u8, 77u8, 152u8, 162u8,
                            53u8, 221u8, 121u8, 176u8, 84u8, 158u8, 169u8, 185u8, 96u8, 85u8,
                            252u8, 56u8, 116u8, 7u8, 46u8, 147u8, 75u8, 194u8, 177u8, 0u8, 252u8,
                        ],
                    )
                }
                #[doc = " Current era's list of reserved validators."]
                pub fn current_era_validators(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::primitives::EraValidators<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Elections",
                        "CurrentEraValidators",
                        vec![],
                        [
                            120u8, 47u8, 38u8, 117u8, 185u8, 231u8, 146u8, 226u8, 139u8, 21u8,
                            230u8, 120u8, 147u8, 157u8, 64u8, 50u8, 153u8, 160u8, 186u8, 53u8,
                            215u8, 8u8, 39u8, 146u8, 195u8, 151u8, 191u8, 0u8, 105u8, 241u8, 152u8,
                            97u8,
                        ],
                    )
                }
                #[doc = " Next era's list of non reserved validators."]
                pub fn next_era_non_reserved_validators(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Elections",
                        "NextEraNonReservedValidators",
                        vec![],
                        [
                            118u8, 139u8, 183u8, 59u8, 75u8, 3u8, 245u8, 47u8, 30u8, 15u8, 23u8,
                            237u8, 0u8, 153u8, 31u8, 251u8, 122u8, 172u8, 215u8, 255u8, 199u8,
                            145u8, 242u8, 3u8, 132u8, 27u8, 20u8, 138u8, 252u8, 235u8, 215u8, 86u8,
                        ],
                    )
                }
                #[doc = " A lookup how many blocks a validator produced."]
                pub fn session_validator_block_count(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Elections",
                        "SessionValidatorBlockCount",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            186u8, 91u8, 185u8, 144u8, 216u8, 179u8, 157u8, 132u8, 17u8, 247u8,
                            241u8, 172u8, 32u8, 7u8, 28u8, 60u8, 188u8, 192u8, 64u8, 29u8, 153u8,
                            100u8, 130u8, 245u8, 189u8, 251u8, 68u8, 161u8, 202u8, 29u8, 153u8,
                            131u8,
                        ],
                    )
                }
                #[doc = " A lookup how many blocks a validator produced."]
                pub fn session_validator_block_count_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Elections",
                        "SessionValidatorBlockCount",
                        Vec::new(),
                        [
                            186u8, 91u8, 185u8, 144u8, 216u8, 179u8, 157u8, 132u8, 17u8, 247u8,
                            241u8, 172u8, 32u8, 7u8, 28u8, 60u8, 188u8, 192u8, 64u8, 29u8, 153u8,
                            100u8, 130u8, 245u8, 189u8, 251u8, 68u8, 161u8, 202u8, 29u8, 153u8,
                            131u8,
                        ],
                    )
                }
                #[doc = " Total possible reward per validator for the current era."]
                pub fn validator_era_total_reward(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_elections::ValidatorTotalRewards<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Elections",
                        "ValidatorEraTotalReward",
                        vec![],
                        [
                            111u8, 103u8, 48u8, 14u8, 23u8, 139u8, 162u8, 122u8, 212u8, 85u8, 64u8,
                            188u8, 36u8, 142u8, 80u8, 224u8, 89u8, 63u8, 104u8, 86u8, 51u8, 111u8,
                            166u8, 53u8, 189u8, 181u8, 240u8, 250u8, 160u8, 128u8, 179u8, 9u8,
                        ],
                    )
                }
                #[doc = " Current era config for ban functionality, see [`BanConfig`]"]
                pub fn ban_config(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::primitives::BanConfig>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Elections",
                        "BanConfig",
                        vec![],
                        [
                            115u8, 228u8, 135u8, 32u8, 85u8, 156u8, 44u8, 195u8, 215u8, 11u8, 27u8,
                            26u8, 231u8, 59u8, 249u8, 78u8, 172u8, 66u8, 81u8, 17u8, 99u8, 221u8,
                            38u8, 253u8, 62u8, 54u8, 104u8, 161u8, 129u8, 92u8, 218u8, 193u8,
                        ],
                    )
                }
                #[doc = " A lookup for a number of underperformance sessions for a given validator"]
                pub fn underperformed_validator_session_count(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Elections",
                        "UnderperformedValidatorSessionCount",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            46u8, 74u8, 46u8, 159u8, 162u8, 118u8, 159u8, 155u8, 233u8, 63u8,
                            101u8, 201u8, 56u8, 204u8, 126u8, 242u8, 131u8, 5u8, 29u8, 132u8, 43u8,
                            205u8, 168u8, 157u8, 29u8, 183u8, 127u8, 202u8, 25u8, 245u8, 137u8,
                            67u8,
                        ],
                    )
                }
                #[doc = " A lookup for a number of underperformance sessions for a given validator"]
                pub fn underperformed_validator_session_count_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Elections",
                        "UnderperformedValidatorSessionCount",
                        Vec::new(),
                        [
                            46u8, 74u8, 46u8, 159u8, 162u8, 118u8, 159u8, 155u8, 233u8, 63u8,
                            101u8, 201u8, 56u8, 204u8, 126u8, 242u8, 131u8, 5u8, 29u8, 132u8, 43u8,
                            205u8, 168u8, 157u8, 29u8, 183u8, 127u8, 202u8, 25u8, 245u8, 137u8,
                            67u8,
                        ],
                    )
                }
                #[doc = " Validators to be removed from non reserved list in the next era"]
                pub fn banned(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::primitives::BanInfo>,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Elections",
                        "Banned",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            202u8, 38u8, 165u8, 35u8, 95u8, 207u8, 116u8, 43u8, 148u8, 73u8, 193u8,
                            187u8, 1u8, 88u8, 209u8, 13u8, 128u8, 168u8, 121u8, 62u8, 227u8, 172u8,
                            87u8, 106u8, 15u8, 43u8, 136u8, 240u8, 249u8, 210u8, 25u8, 215u8,
                        ],
                    )
                }
                #[doc = " Validators to be removed from non reserved list in the next era"]
                pub fn banned_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::primitives::BanInfo>,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Elections",
                        "Banned",
                        Vec::new(),
                        [
                            202u8, 38u8, 165u8, 35u8, 95u8, 207u8, 116u8, 43u8, 148u8, 73u8, 193u8,
                            187u8, 1u8, 88u8, 209u8, 13u8, 128u8, 168u8, 121u8, 62u8, 227u8, 172u8,
                            87u8, 106u8, 15u8, 43u8, 136u8, 240u8, 249u8, 210u8, 25u8, 215u8,
                        ],
                    )
                }
                #[doc = " Openness of the elections, whether we allow all candidates that bonded enough tokens or"]
                #[doc = " the validators list is managed by sudo"]
                pub fn openness(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::primitives::ElectionOpenness,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Elections",
                        "Openness",
                        vec![],
                        [
                            128u8, 231u8, 144u8, 215u8, 195u8, 90u8, 89u8, 180u8, 151u8, 233u8,
                            229u8, 205u8, 40u8, 26u8, 23u8, 134u8, 171u8, 172u8, 140u8, 248u8,
                            172u8, 111u8, 92u8, 51u8, 189u8, 94u8, 91u8, 151u8, 129u8, 248u8, 78u8,
                            12u8,
                        ],
                    )
                }
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                #[doc = " Nr of blocks in the session."]
                pub fn session_period(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Elections",
                        "SessionPeriod",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                #[doc = " Maximum acceptable ban reason length."]
                pub fn maximum_ban_reason_length(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Elections",
                        "MaximumBanReasonLength",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                #[doc = " The maximum number of winners that can be elected by this `ElectionProvider`"]
                #[doc = " implementation."]
                #[doc = ""]
                #[doc = " Note: This must always be greater or equal to `T::DataProvider::desired_targets()`."]
                pub fn max_winners(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Elections",
                        "MaxWinners",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod treasury {
        use super::{root_mod, runtime_types};
        #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ProposeSpend {
                #[codec(compact)]
                pub value: ::core::primitive::u128,
                pub beneficiary: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct RejectProposal {
                #[codec(compact)]
                pub proposal_id: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ApproveProposal {
                #[codec(compact)]
                pub proposal_id: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Spend {
                #[codec(compact)]
                pub amount: ::core::primitive::u128,
                pub beneficiary: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct RemoveApproval {
                #[codec(compact)]
                pub proposal_id: ::core::primitive::u32,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                #[doc = "Put forward a suggestion for spending. A deposit proportional to the value"]
                #[doc = "is reserved and slashed if the proposal is rejected. It is returned once the"]
                #[doc = "proposal is awarded."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- Complexity: O(1)"]
                #[doc = "- DbReads: `ProposalCount`, `origin account`"]
                #[doc = "- DbWrites: `ProposalCount`, `Proposals`, `origin account`"]
                #[doc = "# </weight>"]
                pub fn propose_spend(
                    &self,
                    value: ::core::primitive::u128,
                    beneficiary: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                ) -> ::subxt::tx::StaticTxPayload<ProposeSpend> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Treasury",
                        "propose_spend",
                        ProposeSpend { value, beneficiary },
                        [
                            109u8, 46u8, 8u8, 159u8, 127u8, 79u8, 27u8, 100u8, 92u8, 244u8, 78u8,
                            46u8, 105u8, 246u8, 169u8, 210u8, 149u8, 7u8, 108u8, 153u8, 203u8,
                            223u8, 8u8, 117u8, 126u8, 250u8, 255u8, 52u8, 245u8, 69u8, 45u8, 136u8,
                        ],
                    )
                }
                #[doc = "Reject a proposed spend. The original deposit will be slashed."]
                #[doc = ""]
                #[doc = "May only be called from `T::RejectOrigin`."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- Complexity: O(1)"]
                #[doc = "- DbReads: `Proposals`, `rejected proposer account`"]
                #[doc = "- DbWrites: `Proposals`, `rejected proposer account`"]
                #[doc = "# </weight>"]
                pub fn reject_proposal(
                    &self,
                    proposal_id: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<RejectProposal> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Treasury",
                        "reject_proposal",
                        RejectProposal { proposal_id },
                        [
                            106u8, 223u8, 97u8, 22u8, 111u8, 208u8, 128u8, 26u8, 198u8, 140u8,
                            118u8, 126u8, 187u8, 51u8, 193u8, 50u8, 193u8, 68u8, 143u8, 144u8,
                            34u8, 132u8, 44u8, 244u8, 105u8, 186u8, 223u8, 234u8, 17u8, 145u8,
                            209u8, 145u8,
                        ],
                    )
                }
                #[doc = "Approve a proposal. At a later time, the proposal will be allocated to the beneficiary"]
                #[doc = "and the original deposit will be returned."]
                #[doc = ""]
                #[doc = "May only be called from `T::ApproveOrigin`."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- Complexity: O(1)."]
                #[doc = "- DbReads: `Proposals`, `Approvals`"]
                #[doc = "- DbWrite: `Approvals`"]
                #[doc = "# </weight>"]
                pub fn approve_proposal(
                    &self,
                    proposal_id: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<ApproveProposal> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Treasury",
                        "approve_proposal",
                        ApproveProposal { proposal_id },
                        [
                            164u8, 229u8, 172u8, 98u8, 129u8, 62u8, 84u8, 128u8, 47u8, 108u8, 33u8,
                            120u8, 89u8, 79u8, 57u8, 121u8, 4u8, 197u8, 170u8, 153u8, 156u8, 17u8,
                            59u8, 164u8, 123u8, 227u8, 175u8, 195u8, 220u8, 160u8, 60u8, 186u8,
                        ],
                    )
                }
                #[doc = "Propose and approve a spend of treasury funds."]
                #[doc = ""]
                #[doc = "- `origin`: Must be `SpendOrigin` with the `Success` value being at least `amount`."]
                #[doc = "- `amount`: The amount to be transferred from the treasury to the `beneficiary`."]
                #[doc = "- `beneficiary`: The destination account for the transfer."]
                #[doc = ""]
                #[doc = "NOTE: For record-keeping purposes, the proposer is deemed to be equivalent to the"]
                #[doc = "beneficiary."]
                pub fn spend(
                    &self,
                    amount: ::core::primitive::u128,
                    beneficiary: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                ) -> ::subxt::tx::StaticTxPayload<Spend> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Treasury",
                        "spend",
                        Spend {
                            amount,
                            beneficiary,
                        },
                        [
                            177u8, 178u8, 242u8, 136u8, 135u8, 237u8, 114u8, 71u8, 233u8, 239u8,
                            7u8, 84u8, 14u8, 228u8, 58u8, 31u8, 158u8, 185u8, 25u8, 91u8, 70u8,
                            33u8, 19u8, 92u8, 100u8, 162u8, 5u8, 48u8, 20u8, 120u8, 9u8, 109u8,
                        ],
                    )
                }
                #[doc = "Force a previously approved proposal to be removed from the approval queue."]
                #[doc = "The original deposit will no longer be returned."]
                #[doc = ""]
                #[doc = "May only be called from `T::RejectOrigin`."]
                #[doc = "- `proposal_id`: The index of a proposal"]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- Complexity: O(A) where `A` is the number of approvals"]
                #[doc = "- Db reads and writes: `Approvals`"]
                #[doc = "# </weight>"]
                #[doc = ""]
                #[doc = "Errors:"]
                #[doc = "- `ProposalNotApproved`: The `proposal_id` supplied was not found in the approval queue,"]
                #[doc = "i.e., the proposal has not been approved. This could also mean the proposal does not"]
                #[doc = "exist altogether, thus there is no way it would have been approved in the first place."]
                pub fn remove_approval(
                    &self,
                    proposal_id: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<RemoveApproval> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Treasury",
                        "remove_approval",
                        RemoveApproval { proposal_id },
                        [
                            133u8, 126u8, 181u8, 47u8, 196u8, 243u8, 7u8, 46u8, 25u8, 251u8, 154u8,
                            125u8, 217u8, 77u8, 54u8, 245u8, 240u8, 180u8, 97u8, 34u8, 186u8, 53u8,
                            225u8, 144u8, 155u8, 107u8, 172u8, 54u8, 250u8, 184u8, 178u8, 86u8,
                        ],
                    )
                }
            }
        }
        #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
        pub type Event = runtime_types::pallet_treasury::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: CompactAs,
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "New proposal."]
            pub struct Proposed {
                pub proposal_index: ::core::primitive::u32,
            }
            impl ::subxt::events::StaticEvent for Proposed {
                const PALLET: &'static str = "Treasury";
                const EVENT: &'static str = "Proposed";
            }
            #[derive(
                :: subxt :: ext :: codec :: CompactAs,
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "We have ended a spend period and will now allocate funds."]
            pub struct Spending {
                pub budget_remaining: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Spending {
                const PALLET: &'static str = "Treasury";
                const EVENT: &'static str = "Spending";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Some funds have been allocated."]
            pub struct Awarded {
                pub proposal_index: ::core::primitive::u32,
                pub award: ::core::primitive::u128,
                pub account: ::subxt::ext::sp_core::crypto::AccountId32,
            }
            impl ::subxt::events::StaticEvent for Awarded {
                const PALLET: &'static str = "Treasury";
                const EVENT: &'static str = "Awarded";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A proposal was rejected; funds were slashed."]
            pub struct Rejected {
                pub proposal_index: ::core::primitive::u32,
                pub slashed: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Rejected {
                const PALLET: &'static str = "Treasury";
                const EVENT: &'static str = "Rejected";
            }
            #[derive(
                :: subxt :: ext :: codec :: CompactAs,
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Some of our funds have been burnt."]
            pub struct Burnt {
                pub burnt_funds: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Burnt {
                const PALLET: &'static str = "Treasury";
                const EVENT: &'static str = "Burnt";
            }
            #[derive(
                :: subxt :: ext :: codec :: CompactAs,
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Spending has finished; this is the amount that rolls over until next spend."]
            pub struct Rollover {
                pub rollover_balance: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Rollover {
                const PALLET: &'static str = "Treasury";
                const EVENT: &'static str = "Rollover";
            }
            #[derive(
                :: subxt :: ext :: codec :: CompactAs,
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Some funds have been deposited."]
            pub struct Deposit {
                pub value: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Deposit {
                const PALLET: &'static str = "Treasury";
                const EVENT: &'static str = "Deposit";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A new spend proposal has been approved."]
            pub struct SpendApproved {
                pub proposal_index: ::core::primitive::u32,
                pub amount: ::core::primitive::u128,
                pub beneficiary: ::subxt::ext::sp_core::crypto::AccountId32,
            }
            impl ::subxt::events::StaticEvent for SpendApproved {
                const PALLET: &'static str = "Treasury";
                const EVENT: &'static str = "SpendApproved";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "The inactive funds of the pallet have been updated."]
            pub struct UpdatedInactive {
                pub reactivated: ::core::primitive::u128,
                pub deactivated: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for UpdatedInactive {
                const PALLET: &'static str = "Treasury";
                const EVENT: &'static str = "UpdatedInactive";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                #[doc = " Number of proposals that have been made."]
                pub fn proposal_count(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Treasury",
                        "ProposalCount",
                        vec![],
                        [
                            132u8, 145u8, 78u8, 218u8, 51u8, 189u8, 55u8, 172u8, 143u8, 33u8,
                            140u8, 99u8, 124u8, 208u8, 57u8, 232u8, 154u8, 110u8, 32u8, 142u8,
                            24u8, 149u8, 109u8, 105u8, 30u8, 83u8, 39u8, 177u8, 127u8, 160u8, 34u8,
                            70u8,
                        ],
                    )
                }
                #[doc = " Proposals that have been made."]
                pub fn proposals(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_treasury::Proposal<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            ::core::primitive::u128,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Treasury",
                        "Proposals",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            62u8, 223u8, 55u8, 209u8, 151u8, 134u8, 122u8, 65u8, 207u8, 38u8,
                            113u8, 213u8, 237u8, 48u8, 129u8, 32u8, 91u8, 228u8, 108u8, 91u8, 37u8,
                            49u8, 94u8, 4u8, 75u8, 122u8, 25u8, 34u8, 198u8, 224u8, 246u8, 160u8,
                        ],
                    )
                }
                #[doc = " Proposals that have been made."]
                pub fn proposals_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_treasury::Proposal<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            ::core::primitive::u128,
                        >,
                    >,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Treasury",
                        "Proposals",
                        Vec::new(),
                        [
                            62u8, 223u8, 55u8, 209u8, 151u8, 134u8, 122u8, 65u8, 207u8, 38u8,
                            113u8, 213u8, 237u8, 48u8, 129u8, 32u8, 91u8, 228u8, 108u8, 91u8, 37u8,
                            49u8, 94u8, 4u8, 75u8, 122u8, 25u8, 34u8, 198u8, 224u8, 246u8, 160u8,
                        ],
                    )
                }
                #[doc = " The amount which has been reported as inactive to Currency."]
                pub fn deactivated(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Treasury",
                        "Deactivated",
                        vec![],
                        [
                            159u8, 57u8, 5u8, 85u8, 136u8, 128u8, 70u8, 43u8, 67u8, 76u8, 123u8,
                            206u8, 48u8, 253u8, 51u8, 40u8, 14u8, 35u8, 162u8, 173u8, 127u8, 79u8,
                            38u8, 235u8, 9u8, 141u8, 201u8, 37u8, 211u8, 176u8, 119u8, 106u8,
                        ],
                    )
                }
                #[doc = " Proposal indices that have been approved but not yet awarded."]
                pub fn approvals(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            ::core::primitive::u32,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Treasury",
                        "Approvals",
                        vec![],
                        [
                            202u8, 106u8, 189u8, 40u8, 127u8, 172u8, 108u8, 50u8, 193u8, 4u8,
                            248u8, 226u8, 176u8, 101u8, 212u8, 222u8, 64u8, 206u8, 244u8, 175u8,
                            111u8, 106u8, 86u8, 96u8, 19u8, 109u8, 218u8, 152u8, 30u8, 59u8, 96u8,
                            1u8,
                        ],
                    )
                }
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                #[doc = " Fraction of a proposal's value that should be bonded in order to place the proposal."]
                #[doc = " An accepted proposal gets these back. A rejected proposal does not."]
                pub fn proposal_bond(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_arithmetic::per_things::Permill,
                    >,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Treasury",
                        "ProposalBond",
                        [
                            225u8, 236u8, 95u8, 157u8, 90u8, 94u8, 106u8, 192u8, 254u8, 19u8, 87u8,
                            80u8, 16u8, 62u8, 42u8, 204u8, 136u8, 106u8, 225u8, 53u8, 212u8, 52u8,
                            177u8, 79u8, 4u8, 116u8, 201u8, 104u8, 222u8, 75u8, 86u8, 227u8,
                        ],
                    )
                }
                #[doc = " Minimum amount of funds that should be placed in a deposit for making a proposal."]
                pub fn proposal_bond_minimum(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Treasury",
                        "ProposalBondMinimum",
                        [
                            84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
                            27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
                            136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
                        ],
                    )
                }
                #[doc = " Maximum amount of funds that should be placed in a deposit for making a proposal."]
                pub fn proposal_bond_maximum(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<
                        ::core::option::Option<::core::primitive::u128>,
                    >,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Treasury",
                        "ProposalBondMaximum",
                        [
                            84u8, 154u8, 218u8, 83u8, 84u8, 189u8, 32u8, 20u8, 120u8, 194u8, 88u8,
                            205u8, 109u8, 216u8, 114u8, 193u8, 120u8, 198u8, 154u8, 237u8, 134u8,
                            204u8, 102u8, 247u8, 52u8, 103u8, 231u8, 43u8, 243u8, 122u8, 60u8,
                            216u8,
                        ],
                    )
                }
                #[doc = " Period between successive spends."]
                pub fn spend_period(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Treasury",
                        "SpendPeriod",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                #[doc = " Percentage of spare funds (if any) that are burnt per spend period."]
                pub fn burn(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_arithmetic::per_things::Permill,
                    >,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Treasury",
                        "Burn",
                        [
                            225u8, 236u8, 95u8, 157u8, 90u8, 94u8, 106u8, 192u8, 254u8, 19u8, 87u8,
                            80u8, 16u8, 62u8, 42u8, 204u8, 136u8, 106u8, 225u8, 53u8, 212u8, 52u8,
                            177u8, 79u8, 4u8, 116u8, 201u8, 104u8, 222u8, 75u8, 86u8, 227u8,
                        ],
                    )
                }
                #[doc = " The treasury's pallet id, used for deriving its sovereign account ID."]
                pub fn pallet_id(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::frame_support::PalletId>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Treasury",
                        "PalletId",
                        [
                            139u8, 109u8, 228u8, 151u8, 252u8, 32u8, 130u8, 69u8, 112u8, 154u8,
                            174u8, 45u8, 83u8, 245u8, 51u8, 132u8, 173u8, 5u8, 186u8, 24u8, 243u8,
                            9u8, 12u8, 214u8, 80u8, 74u8, 69u8, 189u8, 30u8, 94u8, 22u8, 39u8,
                        ],
                    )
                }
                #[doc = " The maximum number of approvals that can wait in the spending queue."]
                #[doc = ""]
                #[doc = " NOTE: This parameter is also used within the Bounties Pallet extension if enabled."]
                pub fn max_approvals(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Treasury",
                        "MaxApprovals",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod vesting {
        use super::{root_mod, runtime_types};
        #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Vest;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct VestOther {
                pub target: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct VestedTransfer {
                pub target: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                pub schedule: runtime_types::pallet_vesting::vesting_info::VestingInfo<
                    ::core::primitive::u128,
                    ::core::primitive::u32,
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ForceVestedTransfer {
                pub source: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                pub target: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                pub schedule: runtime_types::pallet_vesting::vesting_info::VestingInfo<
                    ::core::primitive::u128,
                    ::core::primitive::u32,
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct MergeSchedules {
                pub schedule1_index: ::core::primitive::u32,
                pub schedule2_index: ::core::primitive::u32,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                #[doc = "Unlock any vested funds of the sender account."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ and the sender must have funds still"]
                #[doc = "locked under this pallet."]
                #[doc = ""]
                #[doc = "Emits either `VestingCompleted` or `VestingUpdated`."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(1)`."]
                #[doc = "- DbWeight: 2 Reads, 2 Writes"]
                #[doc = "    - Reads: Vesting Storage, Balances Locks, [Sender Account]"]
                #[doc = "    - Writes: Vesting Storage, Balances Locks, [Sender Account]"]
                #[doc = "# </weight>"]
                pub fn vest(&self) -> ::subxt::tx::StaticTxPayload<Vest> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Vesting",
                        "vest",
                        Vest {},
                        [
                            123u8, 54u8, 10u8, 208u8, 154u8, 24u8, 39u8, 166u8, 64u8, 27u8, 74u8,
                            29u8, 243u8, 97u8, 155u8, 5u8, 130u8, 155u8, 65u8, 181u8, 196u8, 125u8,
                            45u8, 133u8, 25u8, 33u8, 3u8, 34u8, 21u8, 167u8, 172u8, 54u8,
                        ],
                    )
                }
                #[doc = "Unlock any vested funds of a `target` account."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_."]
                #[doc = ""]
                #[doc = "- `target`: The account whose vested funds should be unlocked. Must have funds still"]
                #[doc = "locked under this pallet."]
                #[doc = ""]
                #[doc = "Emits either `VestingCompleted` or `VestingUpdated`."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(1)`."]
                #[doc = "- DbWeight: 3 Reads, 3 Writes"]
                #[doc = "    - Reads: Vesting Storage, Balances Locks, Target Account"]
                #[doc = "    - Writes: Vesting Storage, Balances Locks, Target Account"]
                #[doc = "# </weight>"]
                pub fn vest_other(
                    &self,
                    target: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                ) -> ::subxt::tx::StaticTxPayload<VestOther> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Vesting",
                        "vest_other",
                        VestOther { target },
                        [
                            164u8, 19u8, 93u8, 81u8, 235u8, 101u8, 18u8, 52u8, 187u8, 81u8, 243u8,
                            216u8, 116u8, 84u8, 188u8, 135u8, 1u8, 241u8, 128u8, 90u8, 117u8,
                            164u8, 111u8, 0u8, 251u8, 148u8, 250u8, 248u8, 102u8, 79u8, 165u8,
                            175u8,
                        ],
                    )
                }
                #[doc = "Create a vested transfer."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_."]
                #[doc = ""]
                #[doc = "- `target`: The account receiving the vested funds."]
                #[doc = "- `schedule`: The vesting schedule attached to the transfer."]
                #[doc = ""]
                #[doc = "Emits `VestingCreated`."]
                #[doc = ""]
                #[doc = "NOTE: This will unlock all schedules through the current block."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(1)`."]
                #[doc = "- DbWeight: 3 Reads, 3 Writes"]
                #[doc = "    - Reads: Vesting Storage, Balances Locks, Target Account, [Sender Account]"]
                #[doc = "    - Writes: Vesting Storage, Balances Locks, Target Account, [Sender Account]"]
                #[doc = "# </weight>"]
                pub fn vested_transfer(
                    &self,
                    target: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    schedule: runtime_types::pallet_vesting::vesting_info::VestingInfo<
                        ::core::primitive::u128,
                        ::core::primitive::u32,
                    >,
                ) -> ::subxt::tx::StaticTxPayload<VestedTransfer> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Vesting",
                        "vested_transfer",
                        VestedTransfer { target, schedule },
                        [
                            135u8, 172u8, 56u8, 97u8, 45u8, 141u8, 93u8, 173u8, 111u8, 252u8, 75u8,
                            246u8, 92u8, 181u8, 138u8, 87u8, 145u8, 174u8, 71u8, 108u8, 126u8,
                            118u8, 49u8, 122u8, 249u8, 132u8, 19u8, 2u8, 132u8, 160u8, 247u8,
                            195u8,
                        ],
                    )
                }
                #[doc = "Force a vested transfer."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Root_."]
                #[doc = ""]
                #[doc = "- `source`: The account whose funds should be transferred."]
                #[doc = "- `target`: The account that should be transferred the vested funds."]
                #[doc = "- `schedule`: The vesting schedule attached to the transfer."]
                #[doc = ""]
                #[doc = "Emits `VestingCreated`."]
                #[doc = ""]
                #[doc = "NOTE: This will unlock all schedules through the current block."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(1)`."]
                #[doc = "- DbWeight: 4 Reads, 4 Writes"]
                #[doc = "    - Reads: Vesting Storage, Balances Locks, Target Account, Source Account"]
                #[doc = "    - Writes: Vesting Storage, Balances Locks, Target Account, Source Account"]
                #[doc = "# </weight>"]
                pub fn force_vested_transfer(
                    &self,
                    source: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    target: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    schedule: runtime_types::pallet_vesting::vesting_info::VestingInfo<
                        ::core::primitive::u128,
                        ::core::primitive::u32,
                    >,
                ) -> ::subxt::tx::StaticTxPayload<ForceVestedTransfer> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Vesting",
                        "force_vested_transfer",
                        ForceVestedTransfer {
                            source,
                            target,
                            schedule,
                        },
                        [
                            110u8, 142u8, 63u8, 148u8, 90u8, 229u8, 237u8, 183u8, 240u8, 237u8,
                            242u8, 32u8, 88u8, 48u8, 220u8, 101u8, 210u8, 212u8, 27u8, 7u8, 186u8,
                            98u8, 28u8, 197u8, 148u8, 140u8, 77u8, 59u8, 202u8, 166u8, 63u8, 97u8,
                        ],
                    )
                }
                #[doc = "Merge two vesting schedules together, creating a new vesting schedule that unlocks over"]
                #[doc = "the highest possible start and end blocks. If both schedules have already started the"]
                #[doc = "current block will be used as the schedule start; with the caveat that if one schedule"]
                #[doc = "is finished by the current block, the other will be treated as the new merged schedule,"]
                #[doc = "unmodified."]
                #[doc = ""]
                #[doc = "NOTE: If `schedule1_index == schedule2_index` this is a no-op."]
                #[doc = "NOTE: This will unlock all schedules through the current block prior to merging."]
                #[doc = "NOTE: If both schedules have ended by the current block, no new schedule will be created"]
                #[doc = "and both will be removed."]
                #[doc = ""]
                #[doc = "Merged schedule attributes:"]
                #[doc = "- `starting_block`: `MAX(schedule1.starting_block, scheduled2.starting_block,"]
                #[doc = "  current_block)`."]
                #[doc = "- `ending_block`: `MAX(schedule1.ending_block, schedule2.ending_block)`."]
                #[doc = "- `locked`: `schedule1.locked_at(current_block) + schedule2.locked_at(current_block)`."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_."]
                #[doc = ""]
                #[doc = "- `schedule1_index`: index of the first schedule to merge."]
                #[doc = "- `schedule2_index`: index of the second schedule to merge."]
                pub fn merge_schedules(
                    &self,
                    schedule1_index: ::core::primitive::u32,
                    schedule2_index: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<MergeSchedules> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Vesting",
                        "merge_schedules",
                        MergeSchedules {
                            schedule1_index,
                            schedule2_index,
                        },
                        [
                            95u8, 255u8, 147u8, 12u8, 49u8, 25u8, 70u8, 112u8, 55u8, 154u8, 183u8,
                            97u8, 56u8, 244u8, 148u8, 61u8, 107u8, 163u8, 220u8, 31u8, 153u8, 25u8,
                            193u8, 251u8, 131u8, 26u8, 166u8, 157u8, 75u8, 4u8, 110u8, 125u8,
                        ],
                    )
                }
            }
        }
        #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
        pub type Event = runtime_types::pallet_vesting::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "The amount vested has been updated. This could indicate a change in funds available."]
            #[doc = "The balance given is the amount which is left unvested (and thus locked)."]
            pub struct VestingUpdated {
                pub account: ::subxt::ext::sp_core::crypto::AccountId32,
                pub unvested: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for VestingUpdated {
                const PALLET: &'static str = "Vesting";
                const EVENT: &'static str = "VestingUpdated";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "An \\[account\\] has become fully vested."]
            pub struct VestingCompleted {
                pub account: ::subxt::ext::sp_core::crypto::AccountId32,
            }
            impl ::subxt::events::StaticEvent for VestingCompleted {
                const PALLET: &'static str = "Vesting";
                const EVENT: &'static str = "VestingCompleted";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                #[doc = " Information regarding the vesting of a given account."]
                pub fn vesting(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            runtime_types::pallet_vesting::vesting_info::VestingInfo<
                                ::core::primitive::u128,
                                ::core::primitive::u32,
                            >,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Vesting",
                        "Vesting",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Blake2_128Concat,
                        )],
                        [
                            23u8, 209u8, 233u8, 126u8, 89u8, 156u8, 193u8, 204u8, 100u8, 90u8,
                            14u8, 120u8, 36u8, 167u8, 148u8, 239u8, 179u8, 74u8, 207u8, 83u8, 54u8,
                            77u8, 27u8, 135u8, 74u8, 31u8, 33u8, 11u8, 168u8, 239u8, 212u8, 36u8,
                        ],
                    )
                }
                #[doc = " Information regarding the vesting of a given account."]
                pub fn vesting_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            runtime_types::pallet_vesting::vesting_info::VestingInfo<
                                ::core::primitive::u128,
                                ::core::primitive::u32,
                            >,
                        >,
                    >,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Vesting",
                        "Vesting",
                        Vec::new(),
                        [
                            23u8, 209u8, 233u8, 126u8, 89u8, 156u8, 193u8, 204u8, 100u8, 90u8,
                            14u8, 120u8, 36u8, 167u8, 148u8, 239u8, 179u8, 74u8, 207u8, 83u8, 54u8,
                            77u8, 27u8, 135u8, 74u8, 31u8, 33u8, 11u8, 168u8, 239u8, 212u8, 36u8,
                        ],
                    )
                }
                #[doc = " Storage version of the pallet."]
                #[doc = ""]
                #[doc = " New networks start with latest version, as determined by the genesis build."]
                pub fn storage_version(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::pallet_vesting::Releases>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Vesting",
                        "StorageVersion",
                        vec![],
                        [
                            50u8, 143u8, 26u8, 88u8, 129u8, 31u8, 61u8, 118u8, 19u8, 202u8, 119u8,
                            160u8, 34u8, 219u8, 60u8, 57u8, 189u8, 66u8, 93u8, 239u8, 121u8, 114u8,
                            241u8, 116u8, 0u8, 122u8, 232u8, 94u8, 189u8, 23u8, 45u8, 191u8,
                        ],
                    )
                }
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                #[doc = " The minimum amount transferred to call `vested_transfer`."]
                pub fn min_vested_transfer(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Vesting",
                        "MinVestedTransfer",
                        [
                            84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
                            27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
                            136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
                        ],
                    )
                }
                pub fn max_vesting_schedules(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Vesting",
                        "MaxVestingSchedules",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod utility {
        use super::{root_mod, runtime_types};
        #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Batch {
                pub calls: ::std::vec::Vec<runtime_types::aleph_runtime::RuntimeCall>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct AsDerivative {
                pub index: ::core::primitive::u16,
                pub call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct BatchAll {
                pub calls: ::std::vec::Vec<runtime_types::aleph_runtime::RuntimeCall>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct DispatchAs {
                pub as_origin: ::std::boxed::Box<runtime_types::aleph_runtime::OriginCaller>,
                pub call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ForceBatch {
                pub calls: ::std::vec::Vec<runtime_types::aleph_runtime::RuntimeCall>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct WithWeight {
                pub call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
                pub weight: runtime_types::sp_weights::weight_v2::Weight,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                #[doc = "Send a batch of dispatch calls."]
                #[doc = ""]
                #[doc = "May be called from any origin except `None`."]
                #[doc = ""]
                #[doc = "- `calls`: The calls to be dispatched from the same origin. The number of call must not"]
                #[doc = "  exceed the constant: `batched_calls_limit` (available in constant metadata)."]
                #[doc = ""]
                #[doc = "If origin is root then the calls are dispatched without checking origin filter. (This"]
                #[doc = "includes bypassing `frame_system::Config::BaseCallFilter`)."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- Complexity: O(C) where C is the number of calls to be batched."]
                #[doc = "# </weight>"]
                #[doc = ""]
                #[doc = "This will return `Ok` in all circumstances. To determine the success of the batch, an"]
                #[doc = "event is deposited. If a call failed and the batch was interrupted, then the"]
                #[doc = "`BatchInterrupted` event is deposited, along with the number of successful calls made"]
                #[doc = "and the error of the failed call. If all were successful, then the `BatchCompleted`"]
                #[doc = "event is deposited."]
                pub fn batch(
                    &self,
                    calls: ::std::vec::Vec<runtime_types::aleph_runtime::RuntimeCall>,
                ) -> ::subxt::tx::StaticTxPayload<Batch> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Utility",
                        "batch",
                        Batch { calls },
                        [
                            246u8, 95u8, 179u8, 39u8, 249u8, 175u8, 209u8, 240u8, 62u8, 104u8,
                            94u8, 176u8, 39u8, 221u8, 186u8, 130u8, 30u8, 95u8, 148u8, 150u8,
                            164u8, 185u8, 61u8, 206u8, 82u8, 150u8, 21u8, 92u8, 250u8, 42u8, 11u8,
                            84u8,
                        ],
                    )
                }
                #[doc = "Send a call through an indexed pseudonym of the sender."]
                #[doc = ""]
                #[doc = "Filter from origin are passed along. The call will be dispatched with an origin which"]
                #[doc = "use the same filter as the origin of this call."]
                #[doc = ""]
                #[doc = "NOTE: If you need to ensure that any account-based filtering is not honored (i.e."]
                #[doc = "because you expect `proxy` to have been used prior in the call stack and you do not want"]
                #[doc = "the call restrictions to apply to any sub-accounts), then use `as_multi_threshold_1`"]
                #[doc = "in the Multisig pallet instead."]
                #[doc = ""]
                #[doc = "NOTE: Prior to version *12, this was called `as_limited_sub`."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_."]
                pub fn as_derivative(
                    &self,
                    index: ::core::primitive::u16,
                    call: runtime_types::aleph_runtime::RuntimeCall,
                ) -> ::subxt::tx::StaticTxPayload<AsDerivative> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Utility",
                        "as_derivative",
                        AsDerivative {
                            index,
                            call: ::std::boxed::Box::new(call),
                        },
                        [
                            67u8, 121u8, 39u8, 163u8, 227u8, 98u8, 177u8, 171u8, 70u8, 107u8,
                            117u8, 43u8, 251u8, 191u8, 178u8, 195u8, 174u8, 38u8, 113u8, 177u8,
                            246u8, 52u8, 253u8, 234u8, 114u8, 205u8, 158u8, 193u8, 170u8, 109u8,
                            156u8, 90u8,
                        ],
                    )
                }
                #[doc = "Send a batch of dispatch calls and atomically execute them."]
                #[doc = "The whole transaction will rollback and fail if any of the calls failed."]
                #[doc = ""]
                #[doc = "May be called from any origin except `None`."]
                #[doc = ""]
                #[doc = "- `calls`: The calls to be dispatched from the same origin. The number of call must not"]
                #[doc = "  exceed the constant: `batched_calls_limit` (available in constant metadata)."]
                #[doc = ""]
                #[doc = "If origin is root then the calls are dispatched without checking origin filter. (This"]
                #[doc = "includes bypassing `frame_system::Config::BaseCallFilter`)."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- Complexity: O(C) where C is the number of calls to be batched."]
                #[doc = "# </weight>"]
                pub fn batch_all(
                    &self,
                    calls: ::std::vec::Vec<runtime_types::aleph_runtime::RuntimeCall>,
                ) -> ::subxt::tx::StaticTxPayload<BatchAll> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Utility",
                        "batch_all",
                        BatchAll { calls },
                        [
                            232u8, 172u8, 238u8, 163u8, 123u8, 117u8, 137u8, 30u8, 255u8, 209u8,
                            78u8, 112u8, 182u8, 98u8, 131u8, 111u8, 26u8, 148u8, 102u8, 177u8,
                            13u8, 44u8, 155u8, 11u8, 177u8, 178u8, 243u8, 26u8, 194u8, 25u8, 49u8,
                            101u8,
                        ],
                    )
                }
                #[doc = "Dispatches a function call with a provided origin."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Root_."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- O(1)."]
                #[doc = "- Limited storage reads."]
                #[doc = "- One DB write (event)."]
                #[doc = "- Weight of derivative `call` execution + T::WeightInfo::dispatch_as()."]
                #[doc = "# </weight>"]
                pub fn dispatch_as(
                    &self,
                    as_origin: runtime_types::aleph_runtime::OriginCaller,
                    call: runtime_types::aleph_runtime::RuntimeCall,
                ) -> ::subxt::tx::StaticTxPayload<DispatchAs> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Utility",
                        "dispatch_as",
                        DispatchAs {
                            as_origin: ::std::boxed::Box::new(as_origin),
                            call: ::std::boxed::Box::new(call),
                        },
                        [
                            38u8, 191u8, 59u8, 208u8, 154u8, 238u8, 140u8, 43u8, 105u8, 58u8, 64u8,
                            80u8, 205u8, 78u8, 158u8, 48u8, 38u8, 45u8, 167u8, 147u8, 48u8, 196u8,
                            12u8, 19u8, 152u8, 245u8, 107u8, 148u8, 128u8, 145u8, 117u8, 33u8,
                        ],
                    )
                }
                #[doc = "Send a batch of dispatch calls."]
                #[doc = "Unlike `batch`, it allows errors and won't interrupt."]
                #[doc = ""]
                #[doc = "May be called from any origin except `None`."]
                #[doc = ""]
                #[doc = "- `calls`: The calls to be dispatched from the same origin. The number of call must not"]
                #[doc = "  exceed the constant: `batched_calls_limit` (available in constant metadata)."]
                #[doc = ""]
                #[doc = "If origin is root then the calls are dispatch without checking origin filter. (This"]
                #[doc = "includes bypassing `frame_system::Config::BaseCallFilter`)."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- Complexity: O(C) where C is the number of calls to be batched."]
                #[doc = "# </weight>"]
                pub fn force_batch(
                    &self,
                    calls: ::std::vec::Vec<runtime_types::aleph_runtime::RuntimeCall>,
                ) -> ::subxt::tx::StaticTxPayload<ForceBatch> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Utility",
                        "force_batch",
                        ForceBatch { calls },
                        [
                            174u8, 39u8, 96u8, 207u8, 85u8, 73u8, 41u8, 31u8, 24u8, 186u8, 150u8,
                            147u8, 172u8, 151u8, 242u8, 44u8, 226u8, 221u8, 77u8, 249u8, 3u8, 61u8,
                            32u8, 163u8, 149u8, 128u8, 7u8, 112u8, 133u8, 208u8, 138u8, 231u8,
                        ],
                    )
                }
                #[doc = "Dispatch a function call with a specified weight."]
                #[doc = ""]
                #[doc = "This function does not check the weight of the call, and instead allows the"]
                #[doc = "Root origin to specify the weight of the call."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Root_."]
                pub fn with_weight(
                    &self,
                    call: runtime_types::aleph_runtime::RuntimeCall,
                    weight: runtime_types::sp_weights::weight_v2::Weight,
                ) -> ::subxt::tx::StaticTxPayload<WithWeight> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Utility",
                        "with_weight",
                        WithWeight {
                            call: ::std::boxed::Box::new(call),
                            weight,
                        },
                        [
                            59u8, 14u8, 52u8, 140u8, 87u8, 162u8, 12u8, 101u8, 45u8, 45u8, 252u8,
                            72u8, 84u8, 253u8, 68u8, 114u8, 134u8, 188u8, 107u8, 120u8, 177u8,
                            59u8, 0u8, 156u8, 255u8, 22u8, 46u8, 76u8, 22u8, 9u8, 37u8, 246u8,
                        ],
                    )
                }
            }
        }
        #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
        pub type Event = runtime_types::pallet_utility::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Batch of dispatches did not complete fully. Index of first failing dispatch given, as"]
            #[doc = "well as the error."]
            pub struct BatchInterrupted {
                pub index: ::core::primitive::u32,
                pub error: runtime_types::sp_runtime::DispatchError,
            }
            impl ::subxt::events::StaticEvent for BatchInterrupted {
                const PALLET: &'static str = "Utility";
                const EVENT: &'static str = "BatchInterrupted";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Batch of dispatches completed fully with no error."]
            pub struct BatchCompleted;
            impl ::subxt::events::StaticEvent for BatchCompleted {
                const PALLET: &'static str = "Utility";
                const EVENT: &'static str = "BatchCompleted";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Batch of dispatches completed but has errors."]
            pub struct BatchCompletedWithErrors;
            impl ::subxt::events::StaticEvent for BatchCompletedWithErrors {
                const PALLET: &'static str = "Utility";
                const EVENT: &'static str = "BatchCompletedWithErrors";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A single item within a Batch of dispatches has completed with no error."]
            pub struct ItemCompleted;
            impl ::subxt::events::StaticEvent for ItemCompleted {
                const PALLET: &'static str = "Utility";
                const EVENT: &'static str = "ItemCompleted";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A single item within a Batch of dispatches has completed with error."]
            pub struct ItemFailed {
                pub error: runtime_types::sp_runtime::DispatchError,
            }
            impl ::subxt::events::StaticEvent for ItemFailed {
                const PALLET: &'static str = "Utility";
                const EVENT: &'static str = "ItemFailed";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A call was dispatched."]
            pub struct DispatchedAs {
                pub result: ::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
            }
            impl ::subxt::events::StaticEvent for DispatchedAs {
                const PALLET: &'static str = "Utility";
                const EVENT: &'static str = "DispatchedAs";
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                #[doc = " The limit on the number of batched calls."]
                pub fn batched_calls_limit(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Utility",
                        "batched_calls_limit",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod multisig {
        use super::{root_mod, runtime_types};
        #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct AsMultiThreshold1 {
                pub other_signatories: ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                pub call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct AsMulti {
                pub threshold: ::core::primitive::u16,
                pub other_signatories: ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                pub maybe_timepoint: ::core::option::Option<
                    runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
                >,
                pub call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
                pub max_weight: runtime_types::sp_weights::weight_v2::Weight,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ApproveAsMulti {
                pub threshold: ::core::primitive::u16,
                pub other_signatories: ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                pub maybe_timepoint: ::core::option::Option<
                    runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
                >,
                pub call_hash: [::core::primitive::u8; 32usize],
                pub max_weight: runtime_types::sp_weights::weight_v2::Weight,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct CancelAsMulti {
                pub threshold: ::core::primitive::u16,
                pub other_signatories: ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                pub timepoint: runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
                pub call_hash: [::core::primitive::u8; 32usize],
            }
            pub struct TransactionApi;
            impl TransactionApi {
                #[doc = "Immediately dispatch a multi-signature call using a single approval from the caller."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_."]
                #[doc = ""]
                #[doc = "- `other_signatories`: The accounts (other than the sender) who are part of the"]
                #[doc = "multi-signature, but do not participate in the approval process."]
                #[doc = "- `call`: The call to be executed."]
                #[doc = ""]
                #[doc = "Result is equivalent to the dispatched result."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "O(Z + C) where Z is the length of the call and C its execution weight."]
                #[doc = "-------------------------------"]
                #[doc = "- DB Weight: None"]
                #[doc = "- Plus Call Weight"]
                #[doc = "# </weight>"]
                pub fn as_multi_threshold_1(
                    &self,
                    other_signatories: ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                    call: runtime_types::aleph_runtime::RuntimeCall,
                ) -> ::subxt::tx::StaticTxPayload<AsMultiThreshold1> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Multisig",
                        "as_multi_threshold_1",
                        AsMultiThreshold1 {
                            other_signatories,
                            call: ::std::boxed::Box::new(call),
                        },
                        [
                            178u8, 178u8, 110u8, 87u8, 35u8, 86u8, 155u8, 171u8, 44u8, 86u8, 151u8,
                            60u8, 10u8, 245u8, 41u8, 5u8, 156u8, 135u8, 53u8, 252u8, 175u8, 242u8,
                            166u8, 62u8, 135u8, 164u8, 156u8, 50u8, 123u8, 130u8, 45u8, 237u8,
                        ],
                    )
                }
                #[doc = "Register approval for a dispatch to be made from a deterministic composite account if"]
                #[doc = "approved by a total of `threshold - 1` of `other_signatories`."]
                #[doc = ""]
                #[doc = "If there are enough, then dispatch the call."]
                #[doc = ""]
                #[doc = "Payment: `DepositBase` will be reserved if this is the first approval, plus"]
                #[doc = "`threshold` times `DepositFactor`. It is returned once this dispatch happens or"]
                #[doc = "is cancelled."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_."]
                #[doc = ""]
                #[doc = "- `threshold`: The total number of approvals for this dispatch before it is executed."]
                #[doc = "- `other_signatories`: The accounts (other than the sender) who can approve this"]
                #[doc = "dispatch. May not be empty."]
                #[doc = "- `maybe_timepoint`: If this is the first approval, then this must be `None`. If it is"]
                #[doc = "not the first approval, then it must be `Some`, with the timepoint (block number and"]
                #[doc = "transaction index) of the first approval transaction."]
                #[doc = "- `call`: The call to be executed."]
                #[doc = ""]
                #[doc = "NOTE: Unless this is the final approval, you will generally want to use"]
                #[doc = "`approve_as_multi` instead, since it only requires a hash of the call."]
                #[doc = ""]
                #[doc = "Result is equivalent to the dispatched result if `threshold` is exactly `1`. Otherwise"]
                #[doc = "on success, result is `Ok` and the result from the interior call, if it was executed,"]
                #[doc = "may be found in the deposited `MultisigExecuted` event."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(S + Z + Call)`."]
                #[doc = "- Up to one balance-reserve or unreserve operation."]
                #[doc = "- One passthrough operation, one insert, both `O(S)` where `S` is the number of"]
                #[doc = "  signatories. `S` is capped by `MaxSignatories`, with weight being proportional."]
                #[doc = "- One call encode & hash, both of complexity `O(Z)` where `Z` is tx-len."]
                #[doc = "- One encode & hash, both of complexity `O(S)`."]
                #[doc = "- Up to one binary search and insert (`O(logS + S)`)."]
                #[doc = "- I/O: 1 read `O(S)`, up to 1 mutate `O(S)`. Up to one remove."]
                #[doc = "- One event."]
                #[doc = "- The weight of the `call`."]
                #[doc = "- Storage: inserts one item, value size bounded by `MaxSignatories`, with a deposit"]
                #[doc = "  taken for its lifetime of `DepositBase + threshold * DepositFactor`."]
                #[doc = "-------------------------------"]
                #[doc = "- DB Weight:"]
                #[doc = "    - Reads: Multisig Storage, [Caller Account]"]
                #[doc = "    - Writes: Multisig Storage, [Caller Account]"]
                #[doc = "- Plus Call Weight"]
                #[doc = "# </weight>"]
                pub fn as_multi(
                    &self,
                    threshold: ::core::primitive::u16,
                    other_signatories: ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                    maybe_timepoint: ::core::option::Option<
                        runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
                    >,
                    call: runtime_types::aleph_runtime::RuntimeCall,
                    max_weight: runtime_types::sp_weights::weight_v2::Weight,
                ) -> ::subxt::tx::StaticTxPayload<AsMulti> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Multisig",
                        "as_multi",
                        AsMulti {
                            threshold,
                            other_signatories,
                            maybe_timepoint,
                            call: ::std::boxed::Box::new(call),
                            max_weight,
                        },
                        [
                            174u8, 138u8, 25u8, 3u8, 211u8, 169u8, 147u8, 52u8, 165u8, 8u8, 228u8,
                            79u8, 34u8, 2u8, 35u8, 149u8, 151u8, 153u8, 28u8, 91u8, 25u8, 117u8,
                            141u8, 105u8, 20u8, 215u8, 126u8, 255u8, 180u8, 83u8, 143u8, 46u8,
                        ],
                    )
                }
                #[doc = "Register approval for a dispatch to be made from a deterministic composite account if"]
                #[doc = "approved by a total of `threshold - 1` of `other_signatories`."]
                #[doc = ""]
                #[doc = "Payment: `DepositBase` will be reserved if this is the first approval, plus"]
                #[doc = "`threshold` times `DepositFactor`. It is returned once this dispatch happens or"]
                #[doc = "is cancelled."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_."]
                #[doc = ""]
                #[doc = "- `threshold`: The total number of approvals for this dispatch before it is executed."]
                #[doc = "- `other_signatories`: The accounts (other than the sender) who can approve this"]
                #[doc = "dispatch. May not be empty."]
                #[doc = "- `maybe_timepoint`: If this is the first approval, then this must be `None`. If it is"]
                #[doc = "not the first approval, then it must be `Some`, with the timepoint (block number and"]
                #[doc = "transaction index) of the first approval transaction."]
                #[doc = "- `call_hash`: The hash of the call to be executed."]
                #[doc = ""]
                #[doc = "NOTE: If this is the final approval, you will want to use `as_multi` instead."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(S)`."]
                #[doc = "- Up to one balance-reserve or unreserve operation."]
                #[doc = "- One passthrough operation, one insert, both `O(S)` where `S` is the number of"]
                #[doc = "  signatories. `S` is capped by `MaxSignatories`, with weight being proportional."]
                #[doc = "- One encode & hash, both of complexity `O(S)`."]
                #[doc = "- Up to one binary search and insert (`O(logS + S)`)."]
                #[doc = "- I/O: 1 read `O(S)`, up to 1 mutate `O(S)`. Up to one remove."]
                #[doc = "- One event."]
                #[doc = "- Storage: inserts one item, value size bounded by `MaxSignatories`, with a deposit"]
                #[doc = "  taken for its lifetime of `DepositBase + threshold * DepositFactor`."]
                #[doc = "----------------------------------"]
                #[doc = "- DB Weight:"]
                #[doc = "    - Read: Multisig Storage, [Caller Account]"]
                #[doc = "    - Write: Multisig Storage, [Caller Account]"]
                #[doc = "# </weight>"]
                pub fn approve_as_multi(
                    &self,
                    threshold: ::core::primitive::u16,
                    other_signatories: ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                    maybe_timepoint: ::core::option::Option<
                        runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
                    >,
                    call_hash: [::core::primitive::u8; 32usize],
                    max_weight: runtime_types::sp_weights::weight_v2::Weight,
                ) -> ::subxt::tx::StaticTxPayload<ApproveAsMulti> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Multisig",
                        "approve_as_multi",
                        ApproveAsMulti {
                            threshold,
                            other_signatories,
                            maybe_timepoint,
                            call_hash,
                            max_weight,
                        },
                        [
                            133u8, 113u8, 121u8, 66u8, 218u8, 219u8, 48u8, 64u8, 211u8, 114u8,
                            163u8, 193u8, 164u8, 21u8, 140u8, 218u8, 253u8, 237u8, 240u8, 126u8,
                            200u8, 213u8, 184u8, 50u8, 187u8, 182u8, 30u8, 52u8, 142u8, 72u8,
                            210u8, 101u8,
                        ],
                    )
                }
                #[doc = "Cancel a pre-existing, on-going multisig transaction. Any deposit reserved previously"]
                #[doc = "for this operation will be unreserved on success."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_."]
                #[doc = ""]
                #[doc = "- `threshold`: The total number of approvals for this dispatch before it is executed."]
                #[doc = "- `other_signatories`: The accounts (other than the sender) who can approve this"]
                #[doc = "dispatch. May not be empty."]
                #[doc = "- `timepoint`: The timepoint (block number and transaction index) of the first approval"]
                #[doc = "transaction for this dispatch."]
                #[doc = "- `call_hash`: The hash of the call to be executed."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(S)`."]
                #[doc = "- Up to one balance-reserve or unreserve operation."]
                #[doc = "- One passthrough operation, one insert, both `O(S)` where `S` is the number of"]
                #[doc = "  signatories. `S` is capped by `MaxSignatories`, with weight being proportional."]
                #[doc = "- One encode & hash, both of complexity `O(S)`."]
                #[doc = "- One event."]
                #[doc = "- I/O: 1 read `O(S)`, one remove."]
                #[doc = "- Storage: removes one item."]
                #[doc = "----------------------------------"]
                #[doc = "- DB Weight:"]
                #[doc = "    - Read: Multisig Storage, [Caller Account], Refund Account"]
                #[doc = "    - Write: Multisig Storage, [Caller Account], Refund Account"]
                #[doc = "# </weight>"]
                pub fn cancel_as_multi(
                    &self,
                    threshold: ::core::primitive::u16,
                    other_signatories: ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                    timepoint: runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
                    call_hash: [::core::primitive::u8; 32usize],
                ) -> ::subxt::tx::StaticTxPayload<CancelAsMulti> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Multisig",
                        "cancel_as_multi",
                        CancelAsMulti {
                            threshold,
                            other_signatories,
                            timepoint,
                            call_hash,
                        },
                        [
                            30u8, 25u8, 186u8, 142u8, 168u8, 81u8, 235u8, 164u8, 82u8, 209u8, 66u8,
                            129u8, 209u8, 78u8, 172u8, 9u8, 163u8, 222u8, 125u8, 57u8, 2u8, 43u8,
                            169u8, 174u8, 159u8, 167u8, 25u8, 226u8, 254u8, 110u8, 80u8, 216u8,
                        ],
                    )
                }
            }
        }
        #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
        pub type Event = runtime_types::pallet_multisig::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A new multisig operation has begun."]
            pub struct NewMultisig {
                pub approving: ::subxt::ext::sp_core::crypto::AccountId32,
                pub multisig: ::subxt::ext::sp_core::crypto::AccountId32,
                pub call_hash: [::core::primitive::u8; 32usize],
            }
            impl ::subxt::events::StaticEvent for NewMultisig {
                const PALLET: &'static str = "Multisig";
                const EVENT: &'static str = "NewMultisig";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A multisig operation has been approved by someone."]
            pub struct MultisigApproval {
                pub approving: ::subxt::ext::sp_core::crypto::AccountId32,
                pub timepoint: runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
                pub multisig: ::subxt::ext::sp_core::crypto::AccountId32,
                pub call_hash: [::core::primitive::u8; 32usize],
            }
            impl ::subxt::events::StaticEvent for MultisigApproval {
                const PALLET: &'static str = "Multisig";
                const EVENT: &'static str = "MultisigApproval";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A multisig operation has been executed."]
            pub struct MultisigExecuted {
                pub approving: ::subxt::ext::sp_core::crypto::AccountId32,
                pub timepoint: runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
                pub multisig: ::subxt::ext::sp_core::crypto::AccountId32,
                pub call_hash: [::core::primitive::u8; 32usize],
                pub result: ::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
            }
            impl ::subxt::events::StaticEvent for MultisigExecuted {
                const PALLET: &'static str = "Multisig";
                const EVENT: &'static str = "MultisigExecuted";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A multisig operation has been cancelled."]
            pub struct MultisigCancelled {
                pub cancelling: ::subxt::ext::sp_core::crypto::AccountId32,
                pub timepoint: runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
                pub multisig: ::subxt::ext::sp_core::crypto::AccountId32,
                pub call_hash: [::core::primitive::u8; 32usize],
            }
            impl ::subxt::events::StaticEvent for MultisigCancelled {
                const PALLET: &'static str = "Multisig";
                const EVENT: &'static str = "MultisigCancelled";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                #[doc = " The set of open multisig operations."]
                pub fn multisigs(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                    _1: impl ::std::borrow::Borrow<[::core::primitive::u8; 32usize]>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_multisig::Multisig<
                            ::core::primitive::u32,
                            ::core::primitive::u128,
                            ::subxt::ext::sp_core::crypto::AccountId32,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Multisig",
                        "Multisigs",
                        vec![
                            ::subxt::storage::address::StorageMapKey::new(
                                _0.borrow(),
                                ::subxt::storage::address::StorageHasher::Twox64Concat,
                            ),
                            ::subxt::storage::address::StorageMapKey::new(
                                _1.borrow(),
                                ::subxt::storage::address::StorageHasher::Blake2_128Concat,
                            ),
                        ],
                        [
                            69u8, 153u8, 186u8, 204u8, 117u8, 95u8, 119u8, 182u8, 220u8, 87u8, 8u8,
                            15u8, 123u8, 83u8, 5u8, 188u8, 115u8, 121u8, 163u8, 96u8, 218u8, 3u8,
                            106u8, 44u8, 44u8, 187u8, 46u8, 238u8, 80u8, 203u8, 175u8, 155u8,
                        ],
                    )
                }
                #[doc = " The set of open multisig operations."]
                pub fn multisigs_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_multisig::Multisig<
                            ::core::primitive::u32,
                            ::core::primitive::u128,
                            ::subxt::ext::sp_core::crypto::AccountId32,
                        >,
                    >,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Multisig",
                        "Multisigs",
                        Vec::new(),
                        [
                            69u8, 153u8, 186u8, 204u8, 117u8, 95u8, 119u8, 182u8, 220u8, 87u8, 8u8,
                            15u8, 123u8, 83u8, 5u8, 188u8, 115u8, 121u8, 163u8, 96u8, 218u8, 3u8,
                            106u8, 44u8, 44u8, 187u8, 46u8, 238u8, 80u8, 203u8, 175u8, 155u8,
                        ],
                    )
                }
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                #[doc = " The base amount of currency needed to reserve for creating a multisig execution or to"]
                #[doc = " store a dispatch call for later."]
                #[doc = ""]
                #[doc = " This is held for an additional storage item whose value size is"]
                #[doc = " `4 + sizeof((BlockNumber, Balance, AccountId))` bytes and whose key size is"]
                #[doc = " `32 + sizeof(AccountId)` bytes."]
                pub fn deposit_base(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Multisig",
                        "DepositBase",
                        [
                            84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
                            27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
                            136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
                        ],
                    )
                }
                #[doc = " The amount of currency needed per unit threshold when creating a multisig execution."]
                #[doc = ""]
                #[doc = " This is held for adding 32 bytes more into a pre-existing storage value."]
                pub fn deposit_factor(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Multisig",
                        "DepositFactor",
                        [
                            84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
                            27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
                            136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
                        ],
                    )
                }
                #[doc = " The maximum amount of signatories allowed in the multisig."]
                pub fn max_signatories(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Multisig",
                        "MaxSignatories",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod sudo {
        use super::{root_mod, runtime_types};
        #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Sudo {
                pub call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SudoUncheckedWeight {
                pub call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
                pub weight: runtime_types::sp_weights::weight_v2::Weight,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetKey {
                pub new: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SudoAs {
                pub who: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                pub call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                #[doc = "Authenticates the sudo key and dispatches a function call with `Root` origin."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- O(1)."]
                #[doc = "- Limited storage reads."]
                #[doc = "- One DB write (event)."]
                #[doc = "- Weight of derivative `call` execution + 10,000."]
                #[doc = "# </weight>"]
                pub fn sudo(
                    &self,
                    call: runtime_types::aleph_runtime::RuntimeCall,
                ) -> ::subxt::tx::StaticTxPayload<Sudo> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Sudo",
                        "sudo",
                        Sudo {
                            call: ::std::boxed::Box::new(call),
                        },
                        [
                            96u8, 109u8, 155u8, 149u8, 31u8, 0u8, 38u8, 21u8, 17u8, 99u8, 252u8,
                            216u8, 122u8, 10u8, 113u8, 28u8, 148u8, 88u8, 145u8, 164u8, 114u8, 2u8,
                            241u8, 122u8, 48u8, 33u8, 202u8, 244u8, 6u8, 154u8, 154u8, 20u8,
                        ],
                    )
                }
                #[doc = "Authenticates the sudo key and dispatches a function call with `Root` origin."]
                #[doc = "This function does not check the weight of the call, and instead allows the"]
                #[doc = "Sudo user to specify the weight of the call."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- O(1)."]
                #[doc = "- The weight of this call is defined by the caller."]
                #[doc = "# </weight>"]
                pub fn sudo_unchecked_weight(
                    &self,
                    call: runtime_types::aleph_runtime::RuntimeCall,
                    weight: runtime_types::sp_weights::weight_v2::Weight,
                ) -> ::subxt::tx::StaticTxPayload<SudoUncheckedWeight> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Sudo",
                        "sudo_unchecked_weight",
                        SudoUncheckedWeight {
                            call: ::std::boxed::Box::new(call),
                            weight,
                        },
                        [
                            50u8, 68u8, 242u8, 243u8, 214u8, 56u8, 132u8, 15u8, 170u8, 60u8, 186u8,
                            80u8, 15u8, 212u8, 89u8, 227u8, 106u8, 164u8, 116u8, 136u8, 156u8,
                            180u8, 163u8, 197u8, 8u8, 249u8, 166u8, 161u8, 28u8, 197u8, 18u8,
                            146u8,
                        ],
                    )
                }
                #[doc = "Authenticates the current sudo key and sets the given AccountId (`new`) as the new sudo"]
                #[doc = "key."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- O(1)."]
                #[doc = "- Limited storage reads."]
                #[doc = "- One DB change."]
                #[doc = "# </weight>"]
                pub fn set_key(
                    &self,
                    new: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                ) -> ::subxt::tx::StaticTxPayload<SetKey> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Sudo",
                        "set_key",
                        SetKey { new },
                        [
                            23u8, 224u8, 218u8, 169u8, 8u8, 28u8, 111u8, 199u8, 26u8, 88u8, 225u8,
                            105u8, 17u8, 19u8, 87u8, 156u8, 97u8, 67u8, 89u8, 173u8, 70u8, 0u8,
                            5u8, 246u8, 198u8, 135u8, 182u8, 180u8, 44u8, 9u8, 212u8, 95u8,
                        ],
                    )
                }
                #[doc = "Authenticates the sudo key and dispatches a function call with `Signed` origin from"]
                #[doc = "a given account."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- O(1)."]
                #[doc = "- Limited storage reads."]
                #[doc = "- One DB write (event)."]
                #[doc = "- Weight of derivative `call` execution + 10,000."]
                #[doc = "# </weight>"]
                pub fn sudo_as(
                    &self,
                    who: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    call: runtime_types::aleph_runtime::RuntimeCall,
                ) -> ::subxt::tx::StaticTxPayload<SudoAs> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Sudo",
                        "sudo_as",
                        SudoAs {
                            who,
                            call: ::std::boxed::Box::new(call),
                        },
                        [
                            101u8, 206u8, 145u8, 216u8, 78u8, 11u8, 239u8, 246u8, 140u8, 53u8,
                            150u8, 48u8, 92u8, 148u8, 124u8, 169u8, 18u8, 208u8, 229u8, 179u8,
                            219u8, 58u8, 141u8, 100u8, 113u8, 167u8, 91u8, 96u8, 234u8, 43u8, 52u8,
                            185u8,
                        ],
                    )
                }
            }
        }
        #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
        pub type Event = runtime_types::pallet_sudo::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A sudo just took place. \\[result\\]"]
            pub struct Sudid {
                pub sudo_result:
                    ::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
            }
            impl ::subxt::events::StaticEvent for Sudid {
                const PALLET: &'static str = "Sudo";
                const EVENT: &'static str = "Sudid";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "The \\[sudoer\\] just switched identity; the old key is supplied if one existed."]
            pub struct KeyChanged {
                pub old_sudoer: ::core::option::Option<::subxt::ext::sp_core::crypto::AccountId32>,
            }
            impl ::subxt::events::StaticEvent for KeyChanged {
                const PALLET: &'static str = "Sudo";
                const EVENT: &'static str = "KeyChanged";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A sudo just took place. \\[result\\]"]
            pub struct SudoAsDone {
                pub sudo_result:
                    ::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
            }
            impl ::subxt::events::StaticEvent for SudoAsDone {
                const PALLET: &'static str = "Sudo";
                const EVENT: &'static str = "SudoAsDone";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                #[doc = " The `AccountId` of the sudo key."]
                pub fn key(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::subxt::ext::sp_core::crypto::AccountId32>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Sudo",
                        "Key",
                        vec![],
                        [
                            244u8, 73u8, 188u8, 136u8, 218u8, 163u8, 68u8, 179u8, 122u8, 173u8,
                            34u8, 108u8, 137u8, 28u8, 182u8, 16u8, 196u8, 92u8, 138u8, 34u8, 102u8,
                            80u8, 199u8, 88u8, 107u8, 207u8, 36u8, 22u8, 168u8, 167u8, 20u8, 142u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod contracts {
        use super::{root_mod, runtime_types};
        #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct CallOldWeight {
                pub dest: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                #[codec(compact)]
                pub value: ::core::primitive::u128,
                #[codec(compact)]
                pub gas_limit: runtime_types::sp_weights::OldWeight,
                pub storage_deposit_limit:
                    ::core::option::Option<::subxt::ext::codec::Compact<::core::primitive::u128>>,
                pub data: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct InstantiateWithCodeOldWeight {
                #[codec(compact)]
                pub value: ::core::primitive::u128,
                #[codec(compact)]
                pub gas_limit: runtime_types::sp_weights::OldWeight,
                pub storage_deposit_limit:
                    ::core::option::Option<::subxt::ext::codec::Compact<::core::primitive::u128>>,
                pub code: ::std::vec::Vec<::core::primitive::u8>,
                pub data: ::std::vec::Vec<::core::primitive::u8>,
                pub salt: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct InstantiateOldWeight {
                #[codec(compact)]
                pub value: ::core::primitive::u128,
                #[codec(compact)]
                pub gas_limit: runtime_types::sp_weights::OldWeight,
                pub storage_deposit_limit:
                    ::core::option::Option<::subxt::ext::codec::Compact<::core::primitive::u128>>,
                pub code_hash: ::subxt::ext::sp_core::H256,
                pub data: ::std::vec::Vec<::core::primitive::u8>,
                pub salt: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct UploadCode {
                pub code: ::std::vec::Vec<::core::primitive::u8>,
                pub storage_deposit_limit:
                    ::core::option::Option<::subxt::ext::codec::Compact<::core::primitive::u128>>,
                pub determinism: runtime_types::pallet_contracts::wasm::Determinism,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct RemoveCode {
                pub code_hash: ::subxt::ext::sp_core::H256,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetCode {
                pub dest: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                pub code_hash: ::subxt::ext::sp_core::H256,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Call {
                pub dest: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                #[codec(compact)]
                pub value: ::core::primitive::u128,
                pub gas_limit: runtime_types::sp_weights::weight_v2::Weight,
                pub storage_deposit_limit:
                    ::core::option::Option<::subxt::ext::codec::Compact<::core::primitive::u128>>,
                pub data: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct InstantiateWithCode {
                #[codec(compact)]
                pub value: ::core::primitive::u128,
                pub gas_limit: runtime_types::sp_weights::weight_v2::Weight,
                pub storage_deposit_limit:
                    ::core::option::Option<::subxt::ext::codec::Compact<::core::primitive::u128>>,
                pub code: ::std::vec::Vec<::core::primitive::u8>,
                pub data: ::std::vec::Vec<::core::primitive::u8>,
                pub salt: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Instantiate {
                #[codec(compact)]
                pub value: ::core::primitive::u128,
                pub gas_limit: runtime_types::sp_weights::weight_v2::Weight,
                pub storage_deposit_limit:
                    ::core::option::Option<::subxt::ext::codec::Compact<::core::primitive::u128>>,
                pub code_hash: ::subxt::ext::sp_core::H256,
                pub data: ::std::vec::Vec<::core::primitive::u8>,
                pub salt: ::std::vec::Vec<::core::primitive::u8>,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                #[doc = "Deprecated version if [`Self::call`] for use in an in-storage `Call`."]
                pub fn call_old_weight(
                    &self,
                    dest: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    value: ::core::primitive::u128,
                    gas_limit: runtime_types::sp_weights::OldWeight,
                    storage_deposit_limit: ::core::option::Option<
                        ::subxt::ext::codec::Compact<::core::primitive::u128>,
                    >,
                    data: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::StaticTxPayload<CallOldWeight> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Contracts",
                        "call_old_weight",
                        CallOldWeight {
                            dest,
                            value,
                            gas_limit,
                            storage_deposit_limit,
                            data,
                        },
                        [
                            181u8, 255u8, 119u8, 227u8, 10u8, 39u8, 128u8, 22u8, 223u8, 250u8,
                            247u8, 253u8, 118u8, 113u8, 192u8, 65u8, 224u8, 0u8, 93u8, 16u8, 41u8,
                            177u8, 150u8, 70u8, 151u8, 216u8, 76u8, 97u8, 27u8, 127u8, 75u8, 67u8,
                        ],
                    )
                }
                #[doc = "Deprecated version if [`Self::instantiate_with_code`] for use in an in-storage `Call`."]
                pub fn instantiate_with_code_old_weight(
                    &self,
                    value: ::core::primitive::u128,
                    gas_limit: runtime_types::sp_weights::OldWeight,
                    storage_deposit_limit: ::core::option::Option<
                        ::subxt::ext::codec::Compact<::core::primitive::u128>,
                    >,
                    code: ::std::vec::Vec<::core::primitive::u8>,
                    data: ::std::vec::Vec<::core::primitive::u8>,
                    salt: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::StaticTxPayload<InstantiateWithCodeOldWeight> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Contracts",
                        "instantiate_with_code_old_weight",
                        InstantiateWithCodeOldWeight {
                            value,
                            gas_limit,
                            storage_deposit_limit,
                            code,
                            data,
                            salt,
                        },
                        [
                            93u8, 124u8, 100u8, 101u8, 7u8, 110u8, 92u8, 199u8, 162u8, 126u8, 35u8,
                            47u8, 190u8, 42u8, 237u8, 152u8, 169u8, 130u8, 21u8, 33u8, 136u8,
                            220u8, 110u8, 106u8, 57u8, 211u8, 158u8, 130u8, 112u8, 37u8, 41u8,
                            39u8,
                        ],
                    )
                }
                #[doc = "Deprecated version if [`Self::instantiate`] for use in an in-storage `Call`."]
                pub fn instantiate_old_weight(
                    &self,
                    value: ::core::primitive::u128,
                    gas_limit: runtime_types::sp_weights::OldWeight,
                    storage_deposit_limit: ::core::option::Option<
                        ::subxt::ext::codec::Compact<::core::primitive::u128>,
                    >,
                    code_hash: ::subxt::ext::sp_core::H256,
                    data: ::std::vec::Vec<::core::primitive::u8>,
                    salt: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::StaticTxPayload<InstantiateOldWeight> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Contracts",
                        "instantiate_old_weight",
                        InstantiateOldWeight {
                            value,
                            gas_limit,
                            storage_deposit_limit,
                            code_hash,
                            data,
                            salt,
                        },
                        [
                            243u8, 56u8, 93u8, 198u8, 169u8, 134u8, 6u8, 135u8, 19u8, 1u8, 20u8,
                            138u8, 202u8, 59u8, 59u8, 99u8, 58u8, 22u8, 33u8, 94u8, 253u8, 215u8,
                            203u8, 159u8, 58u8, 21u8, 24u8, 235u8, 30u8, 215u8, 173u8, 23u8,
                        ],
                    )
                }
                #[doc = "Upload new `code` without instantiating a contract from it."]
                #[doc = ""]
                #[doc = "If the code does not already exist a deposit is reserved from the caller"]
                #[doc = "and unreserved only when [`Self::remove_code`] is called. The size of the reserve"]
                #[doc = "depends on the instrumented size of the the supplied `code`."]
                #[doc = ""]
                #[doc = "If the code already exists in storage it will still return `Ok` and upgrades"]
                #[doc = "the in storage version to the current"]
                #[doc = "[`InstructionWeights::version`](InstructionWeights)."]
                #[doc = ""]
                #[doc = "- `determinism`: If this is set to any other value but [`Determinism::Deterministic`]"]
                #[doc = "  then the only way to use this code is to delegate call into it from an offchain"]
                #[doc = "  execution. Set to [`Determinism::Deterministic`] if in doubt."]
                #[doc = ""]
                #[doc = "# Note"]
                #[doc = ""]
                #[doc = "Anyone can instantiate a contract from any uploaded code and thus prevent its removal."]
                #[doc = "To avoid this situation a constructor could employ access control so that it can"]
                #[doc = "only be instantiated by permissioned entities. The same is true when uploading"]
                #[doc = "through [`Self::instantiate_with_code`]."]
                pub fn upload_code(
                    &self,
                    code: ::std::vec::Vec<::core::primitive::u8>,
                    storage_deposit_limit: ::core::option::Option<
                        ::subxt::ext::codec::Compact<::core::primitive::u128>,
                    >,
                    determinism: runtime_types::pallet_contracts::wasm::Determinism,
                ) -> ::subxt::tx::StaticTxPayload<UploadCode> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Contracts",
                        "upload_code",
                        UploadCode {
                            code,
                            storage_deposit_limit,
                            determinism,
                        },
                        [
                            233u8, 137u8, 54u8, 111u8, 132u8, 124u8, 80u8, 213u8, 182u8, 224u8,
                            144u8, 240u8, 6u8, 235u8, 148u8, 26u8, 65u8, 39u8, 91u8, 151u8, 131u8,
                            10u8, 216u8, 101u8, 89u8, 115u8, 160u8, 154u8, 44u8, 239u8, 142u8,
                            116u8,
                        ],
                    )
                }
                #[doc = "Remove the code stored under `code_hash` and refund the deposit to its owner."]
                #[doc = ""]
                #[doc = "A code can only be removed by its original uploader (its owner) and only if it is"]
                #[doc = "not used by any contract."]
                pub fn remove_code(
                    &self,
                    code_hash: ::subxt::ext::sp_core::H256,
                ) -> ::subxt::tx::StaticTxPayload<RemoveCode> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Contracts",
                        "remove_code",
                        RemoveCode { code_hash },
                        [
                            43u8, 192u8, 198u8, 182u8, 108u8, 76u8, 21u8, 42u8, 169u8, 41u8, 195u8,
                            73u8, 31u8, 179u8, 162u8, 56u8, 91u8, 5u8, 64u8, 7u8, 252u8, 194u8,
                            255u8, 170u8, 67u8, 137u8, 143u8, 192u8, 2u8, 149u8, 38u8, 180u8,
                        ],
                    )
                }
                #[doc = "Privileged function that changes the code of an existing contract."]
                #[doc = ""]
                #[doc = "This takes care of updating refcounts and all other necessary operations. Returns"]
                #[doc = "an error if either the `code_hash` or `dest` do not exist."]
                #[doc = ""]
                #[doc = "# Note"]
                #[doc = ""]
                #[doc = "This does **not** change the address of the contract in question. This means"]
                #[doc = "that the contract address is no longer derived from its code hash after calling"]
                #[doc = "this dispatchable."]
                pub fn set_code(
                    &self,
                    dest: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    code_hash: ::subxt::ext::sp_core::H256,
                ) -> ::subxt::tx::StaticTxPayload<SetCode> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Contracts",
                        "set_code",
                        SetCode { dest, code_hash },
                        [
                            106u8, 141u8, 239u8, 113u8, 99u8, 74u8, 14u8, 171u8, 80u8, 115u8,
                            214u8, 203u8, 232u8, 142u8, 48u8, 207u8, 214u8, 59u8, 204u8, 157u8,
                            101u8, 142u8, 12u8, 69u8, 230u8, 188u8, 60u8, 197u8, 238u8, 146u8,
                            17u8, 190u8,
                        ],
                    )
                }
                #[doc = "Makes a call to an account, optionally transferring some balance."]
                #[doc = ""]
                #[doc = "# Parameters"]
                #[doc = ""]
                #[doc = "* `dest`: Address of the contract to call."]
                #[doc = "* `value`: The balance to transfer from the `origin` to `dest`."]
                #[doc = "* `gas_limit`: The gas limit enforced when executing the constructor."]
                #[doc = "* `storage_deposit_limit`: The maximum amount of balance that can be charged from the"]
                #[doc = "  caller to pay for the storage consumed."]
                #[doc = "* `data`: The input data to pass to the contract."]
                #[doc = ""]
                #[doc = "* If the account is a smart-contract account, the associated code will be"]
                #[doc = "executed and any value will be transferred."]
                #[doc = "* If the account is a regular account, any value will be transferred."]
                #[doc = "* If no account exists and the call value is not less than `existential_deposit`,"]
                #[doc = "a regular account will be created and any value will be transferred."]
                pub fn call(
                    &self,
                    dest: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    value: ::core::primitive::u128,
                    gas_limit: runtime_types::sp_weights::weight_v2::Weight,
                    storage_deposit_limit: ::core::option::Option<
                        ::subxt::ext::codec::Compact<::core::primitive::u128>,
                    >,
                    data: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::StaticTxPayload<Call> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Contracts",
                        "call",
                        Call {
                            dest,
                            value,
                            gas_limit,
                            storage_deposit_limit,
                            data,
                        },
                        [
                            226u8, 219u8, 120u8, 119u8, 106u8, 251u8, 205u8, 112u8, 148u8, 215u8,
                            196u8, 50u8, 116u8, 75u8, 40u8, 41u8, 224u8, 35u8, 186u8, 29u8, 49u8,
                            112u8, 51u8, 117u8, 142u8, 69u8, 214u8, 208u8, 241u8, 71u8, 149u8,
                            163u8,
                        ],
                    )
                }
                #[doc = "Instantiates a new contract from the supplied `code` optionally transferring"]
                #[doc = "some balance."]
                #[doc = ""]
                #[doc = "This dispatchable has the same effect as calling [`Self::upload_code`] +"]
                #[doc = "[`Self::instantiate`]. Bundling them together provides efficiency gains. Please"]
                #[doc = "also check the documentation of [`Self::upload_code`]."]
                #[doc = ""]
                #[doc = "# Parameters"]
                #[doc = ""]
                #[doc = "* `value`: The balance to transfer from the `origin` to the newly created contract."]
                #[doc = "* `gas_limit`: The gas limit enforced when executing the constructor."]
                #[doc = "* `storage_deposit_limit`: The maximum amount of balance that can be charged/reserved"]
                #[doc = "  from the caller to pay for the storage consumed."]
                #[doc = "* `code`: The contract code to deploy in raw bytes."]
                #[doc = "* `data`: The input data to pass to the contract constructor."]
                #[doc = "* `salt`: Used for the address derivation. See [`Pallet::contract_address`]."]
                #[doc = ""]
                #[doc = "Instantiation is executed as follows:"]
                #[doc = ""]
                #[doc = "- The supplied `code` is instrumented, deployed, and a `code_hash` is created for that"]
                #[doc = "  code."]
                #[doc = "- If the `code_hash` already exists on the chain the underlying `code` will be shared."]
                #[doc = "- The destination address is computed based on the sender, code_hash and the salt."]
                #[doc = "- The smart-contract account is created at the computed address."]
                #[doc = "- The `value` is transferred to the new account."]
                #[doc = "- The `deploy` function is executed in the context of the newly-created account."]
                pub fn instantiate_with_code(
                    &self,
                    value: ::core::primitive::u128,
                    gas_limit: runtime_types::sp_weights::weight_v2::Weight,
                    storage_deposit_limit: ::core::option::Option<
                        ::subxt::ext::codec::Compact<::core::primitive::u128>,
                    >,
                    code: ::std::vec::Vec<::core::primitive::u8>,
                    data: ::std::vec::Vec<::core::primitive::u8>,
                    salt: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::StaticTxPayload<InstantiateWithCode> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Contracts",
                        "instantiate_with_code",
                        InstantiateWithCode {
                            value,
                            gas_limit,
                            storage_deposit_limit,
                            code,
                            data,
                            salt,
                        },
                        [
                            94u8, 238u8, 175u8, 86u8, 230u8, 186u8, 94u8, 60u8, 201u8, 35u8, 117u8,
                            236u8, 221u8, 10u8, 180u8, 191u8, 140u8, 79u8, 203u8, 134u8, 240u8,
                            21u8, 31u8, 63u8, 9u8, 17u8, 134u8, 30u8, 244u8, 95u8, 171u8, 164u8,
                        ],
                    )
                }
                #[doc = "Instantiates a contract from a previously deployed wasm binary."]
                #[doc = ""]
                #[doc = "This function is identical to [`Self::instantiate_with_code`] but without the"]
                #[doc = "code deployment step. Instead, the `code_hash` of an on-chain deployed wasm binary"]
                #[doc = "must be supplied."]
                pub fn instantiate(
                    &self,
                    value: ::core::primitive::u128,
                    gas_limit: runtime_types::sp_weights::weight_v2::Weight,
                    storage_deposit_limit: ::core::option::Option<
                        ::subxt::ext::codec::Compact<::core::primitive::u128>,
                    >,
                    code_hash: ::subxt::ext::sp_core::H256,
                    data: ::std::vec::Vec<::core::primitive::u8>,
                    salt: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::StaticTxPayload<Instantiate> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Contracts",
                        "instantiate",
                        Instantiate {
                            value,
                            gas_limit,
                            storage_deposit_limit,
                            code_hash,
                            data,
                            salt,
                        },
                        [
                            251u8, 49u8, 158u8, 1u8, 138u8, 29u8, 106u8, 187u8, 68u8, 135u8, 44u8,
                            196u8, 230u8, 237u8, 88u8, 244u8, 170u8, 168u8, 11u8, 91u8, 185u8,
                            11u8, 45u8, 86u8, 113u8, 79u8, 92u8, 248u8, 113u8, 47u8, 141u8, 10u8,
                        ],
                    )
                }
            }
        }
        #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
        pub type Event = runtime_types::pallet_contracts::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Contract deployed by address at the specified address."]
            pub struct Instantiated {
                pub deployer: ::subxt::ext::sp_core::crypto::AccountId32,
                pub contract: ::subxt::ext::sp_core::crypto::AccountId32,
            }
            impl ::subxt::events::StaticEvent for Instantiated {
                const PALLET: &'static str = "Contracts";
                const EVENT: &'static str = "Instantiated";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Contract has been removed."]
            #[doc = ""]
            #[doc = "# Note"]
            #[doc = ""]
            #[doc = "The only way for a contract to be removed and emitting this event is by calling"]
            #[doc = "`seal_terminate`."]
            pub struct Terminated {
                pub contract: ::subxt::ext::sp_core::crypto::AccountId32,
                pub beneficiary: ::subxt::ext::sp_core::crypto::AccountId32,
            }
            impl ::subxt::events::StaticEvent for Terminated {
                const PALLET: &'static str = "Contracts";
                const EVENT: &'static str = "Terminated";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Code with the specified hash has been stored."]
            pub struct CodeStored {
                pub code_hash: ::subxt::ext::sp_core::H256,
            }
            impl ::subxt::events::StaticEvent for CodeStored {
                const PALLET: &'static str = "Contracts";
                const EVENT: &'static str = "CodeStored";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A custom event emitted by the contract."]
            pub struct ContractEmitted {
                pub contract: ::subxt::ext::sp_core::crypto::AccountId32,
                pub data: ::std::vec::Vec<::core::primitive::u8>,
            }
            impl ::subxt::events::StaticEvent for ContractEmitted {
                const PALLET: &'static str = "Contracts";
                const EVENT: &'static str = "ContractEmitted";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A code with the specified hash was removed."]
            pub struct CodeRemoved {
                pub code_hash: ::subxt::ext::sp_core::H256,
            }
            impl ::subxt::events::StaticEvent for CodeRemoved {
                const PALLET: &'static str = "Contracts";
                const EVENT: &'static str = "CodeRemoved";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A contract's code was updated."]
            pub struct ContractCodeUpdated {
                pub contract: ::subxt::ext::sp_core::crypto::AccountId32,
                pub new_code_hash: ::subxt::ext::sp_core::H256,
                pub old_code_hash: ::subxt::ext::sp_core::H256,
            }
            impl ::subxt::events::StaticEvent for ContractCodeUpdated {
                const PALLET: &'static str = "Contracts";
                const EVENT: &'static str = "ContractCodeUpdated";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A contract was called either by a plain account or another contract."]
            #[doc = ""]
            #[doc = "# Note"]
            #[doc = ""]
            #[doc = "Please keep in mind that like all events this is only emitted for successful"]
            #[doc = "calls. This is because on failure all storage changes including events are"]
            #[doc = "rolled back."]
            pub struct Called {
                pub caller: ::subxt::ext::sp_core::crypto::AccountId32,
                pub contract: ::subxt::ext::sp_core::crypto::AccountId32,
            }
            impl ::subxt::events::StaticEvent for Called {
                const PALLET: &'static str = "Contracts";
                const EVENT: &'static str = "Called";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A contract delegate called a code hash."]
            #[doc = ""]
            #[doc = "# Note"]
            #[doc = ""]
            #[doc = "Please keep in mind that like all events this is only emitted for successful"]
            #[doc = "calls. This is because on failure all storage changes including events are"]
            #[doc = "rolled back."]
            pub struct DelegateCalled {
                pub contract: ::subxt::ext::sp_core::crypto::AccountId32,
                pub code_hash: ::subxt::ext::sp_core::H256,
            }
            impl ::subxt::events::StaticEvent for DelegateCalled {
                const PALLET: &'static str = "Contracts";
                const EVENT: &'static str = "DelegateCalled";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                #[doc = " A mapping from an original code hash to the original code, untouched by instrumentation."]
                pub fn pristine_code(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::H256>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            ::core::primitive::u8,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Contracts",
                        "PristineCode",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Identity,
                        )],
                        [
                            244u8, 169u8, 220u8, 235u8, 62u8, 153u8, 226u8, 187u8, 220u8, 141u8,
                            149u8, 75u8, 224u8, 117u8, 181u8, 147u8, 140u8, 84u8, 9u8, 109u8,
                            230u8, 25u8, 186u8, 26u8, 171u8, 147u8, 19u8, 78u8, 62u8, 170u8, 27u8,
                            105u8,
                        ],
                    )
                }
                #[doc = " A mapping from an original code hash to the original code, untouched by instrumentation."]
                pub fn pristine_code_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            ::core::primitive::u8,
                        >,
                    >,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Contracts",
                        "PristineCode",
                        Vec::new(),
                        [
                            244u8, 169u8, 220u8, 235u8, 62u8, 153u8, 226u8, 187u8, 220u8, 141u8,
                            149u8, 75u8, 224u8, 117u8, 181u8, 147u8, 140u8, 84u8, 9u8, 109u8,
                            230u8, 25u8, 186u8, 26u8, 171u8, 147u8, 19u8, 78u8, 62u8, 170u8, 27u8,
                            105u8,
                        ],
                    )
                }
                #[doc = " A mapping between an original code hash and instrumented wasm code, ready for execution."]
                pub fn code_storage(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::H256>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_contracts::wasm::PrefabWasmModule,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Contracts",
                        "CodeStorage",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Identity,
                        )],
                        [
                            57u8, 55u8, 36u8, 82u8, 39u8, 194u8, 172u8, 147u8, 144u8, 63u8, 101u8,
                            240u8, 179u8, 25u8, 177u8, 68u8, 253u8, 230u8, 156u8, 228u8, 181u8,
                            194u8, 48u8, 99u8, 188u8, 117u8, 44u8, 80u8, 121u8, 46u8, 149u8, 48u8,
                        ],
                    )
                }
                #[doc = " A mapping between an original code hash and instrumented wasm code, ready for execution."]
                pub fn code_storage_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_contracts::wasm::PrefabWasmModule,
                    >,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Contracts",
                        "CodeStorage",
                        Vec::new(),
                        [
                            57u8, 55u8, 36u8, 82u8, 39u8, 194u8, 172u8, 147u8, 144u8, 63u8, 101u8,
                            240u8, 179u8, 25u8, 177u8, 68u8, 253u8, 230u8, 156u8, 228u8, 181u8,
                            194u8, 48u8, 99u8, 188u8, 117u8, 44u8, 80u8, 121u8, 46u8, 149u8, 48u8,
                        ],
                    )
                }
                #[doc = " A mapping between an original code hash and its owner information."]
                pub fn owner_info_of(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::H256>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_contracts::wasm::OwnerInfo,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Contracts",
                        "OwnerInfoOf",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Identity,
                        )],
                        [
                            147u8, 6u8, 225u8, 62u8, 211u8, 236u8, 61u8, 116u8, 152u8, 219u8,
                            220u8, 17u8, 82u8, 221u8, 156u8, 88u8, 63u8, 204u8, 16u8, 11u8, 184u8,
                            236u8, 181u8, 189u8, 170u8, 160u8, 60u8, 64u8, 71u8, 250u8, 202u8,
                            186u8,
                        ],
                    )
                }
                #[doc = " A mapping between an original code hash and its owner information."]
                pub fn owner_info_of_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_contracts::wasm::OwnerInfo,
                    >,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Contracts",
                        "OwnerInfoOf",
                        Vec::new(),
                        [
                            147u8, 6u8, 225u8, 62u8, 211u8, 236u8, 61u8, 116u8, 152u8, 219u8,
                            220u8, 17u8, 82u8, 221u8, 156u8, 88u8, 63u8, 204u8, 16u8, 11u8, 184u8,
                            236u8, 181u8, 189u8, 170u8, 160u8, 60u8, 64u8, 71u8, 250u8, 202u8,
                            186u8,
                        ],
                    )
                }
                #[doc = " This is a **monotonic** counter incremented on contract instantiation."]
                #[doc = ""]
                #[doc = " This is used in order to generate unique trie ids for contracts."]
                #[doc = " The trie id of a new contract is calculated from hash(account_id, nonce)."]
                #[doc = " The nonce is required because otherwise the following sequence would lead to"]
                #[doc = " a possible collision of storage:"]
                #[doc = ""]
                #[doc = " 1. Create a new contract."]
                #[doc = " 2. Terminate the contract."]
                #[doc = " 3. Immediately recreate the contract with the same account_id."]
                #[doc = ""]
                #[doc = " This is bad because the contents of a trie are deleted lazily and there might be"]
                #[doc = " storage of the old instantiation still in it when the new contract is created. Please"]
                #[doc = " note that we can't replace the counter by the block number because the sequence above"]
                #[doc = " can happen in the same block. We also can't keep the account counter in memory only"]
                #[doc = " because storage is the only way to communicate across different extrinsics in the"]
                #[doc = " same block."]
                #[doc = ""]
                #[doc = " # Note"]
                #[doc = ""]
                #[doc = " Do not use it to determine the number of contracts. It won't be decremented if"]
                #[doc = " a contract is destroyed."]
                pub fn nonce(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u64>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Contracts",
                        "Nonce",
                        vec![],
                        [
                            122u8, 169u8, 95u8, 131u8, 85u8, 32u8, 154u8, 114u8, 143u8, 56u8, 12u8,
                            182u8, 64u8, 150u8, 241u8, 249u8, 254u8, 251u8, 160u8, 235u8, 192u8,
                            41u8, 101u8, 232u8, 186u8, 108u8, 187u8, 149u8, 210u8, 91u8, 179u8,
                            98u8,
                        ],
                    )
                }
                #[doc = " The code associated with a given account."]
                #[doc = ""]
                #[doc = " TWOX-NOTE: SAFE since `AccountId` is a secure hash."]
                pub fn contract_info_of(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_contracts::storage::ContractInfo,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Contracts",
                        "ContractInfoOf",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            176u8, 73u8, 209u8, 119u8, 242u8, 147u8, 64u8, 203u8, 253u8, 178u8,
                            8u8, 239u8, 64u8, 68u8, 106u8, 153u8, 28u8, 124u8, 52u8, 226u8, 67u8,
                            54u8, 177u8, 206u8, 238u8, 179u8, 222u8, 225u8, 242u8, 0u8, 171u8,
                            184u8,
                        ],
                    )
                }
                #[doc = " The code associated with a given account."]
                #[doc = ""]
                #[doc = " TWOX-NOTE: SAFE since `AccountId` is a secure hash."]
                pub fn contract_info_of_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_contracts::storage::ContractInfo,
                    >,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Contracts",
                        "ContractInfoOf",
                        Vec::new(),
                        [
                            176u8, 73u8, 209u8, 119u8, 242u8, 147u8, 64u8, 203u8, 253u8, 178u8,
                            8u8, 239u8, 64u8, 68u8, 106u8, 153u8, 28u8, 124u8, 52u8, 226u8, 67u8,
                            54u8, 177u8, 206u8, 238u8, 179u8, 222u8, 225u8, 242u8, 0u8, 171u8,
                            184u8,
                        ],
                    )
                }
                #[doc = " Evicted contracts that await child trie deletion."]
                #[doc = ""]
                #[doc = " Child trie deletion is a heavy operation depending on the amount of storage items"]
                #[doc = " stored in said trie. Therefore this operation is performed lazily in `on_initialize`."]
                pub fn deletion_queue(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            runtime_types::pallet_contracts::storage::DeletedContract,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Contracts",
                        "DeletionQueue",
                        vec![],
                        [
                            119u8, 169u8, 146u8, 210u8, 21u8, 216u8, 51u8, 225u8, 107u8, 61u8,
                            42u8, 155u8, 169u8, 127u8, 140u8, 106u8, 255u8, 137u8, 163u8, 199u8,
                            91u8, 137u8, 73u8, 61u8, 9u8, 167u8, 16u8, 157u8, 183u8, 212u8, 35u8,
                            88u8,
                        ],
                    )
                }
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                #[doc = " Cost schedule and limits."]
                pub fn schedule(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_contracts::schedule::Schedule,
                    >,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Contracts",
                        "Schedule",
                        [
                            102u8, 52u8, 108u8, 178u8, 197u8, 144u8, 39u8, 115u8, 254u8, 23u8,
                            38u8, 120u8, 11u8, 166u8, 178u8, 210u8, 91u8, 139u8, 214u8, 231u8,
                            110u8, 188u8, 37u8, 149u8, 195u8, 73u8, 166u8, 90u8, 55u8, 73u8, 88u8,
                            111u8,
                        ],
                    )
                }
                #[doc = " The maximum number of contracts that can be pending for deletion."]
                #[doc = ""]
                #[doc = " When a contract is deleted by calling `seal_terminate` it becomes inaccessible"]
                #[doc = " immediately, but the deletion of the storage items it has accumulated is performed"]
                #[doc = " later. The contract is put into the deletion queue. This defines how many"]
                #[doc = " contracts can be queued up at the same time. If that limit is reached `seal_terminate`"]
                #[doc = " will fail. The action must be retried in a later block in that case."]
                #[doc = ""]
                #[doc = " The reasons for limiting the queue depth are:"]
                #[doc = ""]
                #[doc = " 1. The queue is in storage in order to be persistent between blocks. We want to limit"]
                #[doc = " \tthe amount of storage that can be consumed."]
                #[doc = " 2. The queue is stored in a vector and needs to be decoded as a whole when reading"]
                #[doc = "\t\tit at the end of each block. Longer queues take more weight to decode and hence"]
                #[doc = "\t\tlimit the amount of items that can be deleted per block."]
                pub fn deletion_queue_depth(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Contracts",
                        "DeletionQueueDepth",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                #[doc = " The maximum amount of weight that can be consumed per block for lazy trie removal."]
                #[doc = ""]
                #[doc = " The amount of weight that is dedicated per block to work on the deletion queue. Larger"]
                #[doc = " values allow more trie keys to be deleted in each block but reduce the amount of"]
                #[doc = " weight that is left for transactions. See [`Self::DeletionQueueDepth`] for more"]
                #[doc = " information about the deletion queue."]
                pub fn deletion_weight_limit(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_weights::weight_v2::Weight,
                    >,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Contracts",
                        "DeletionWeightLimit",
                        [
                            206u8, 61u8, 253u8, 247u8, 163u8, 40u8, 161u8, 52u8, 134u8, 140u8,
                            206u8, 83u8, 44u8, 166u8, 226u8, 115u8, 181u8, 14u8, 227u8, 130u8,
                            210u8, 32u8, 85u8, 29u8, 230u8, 97u8, 130u8, 165u8, 147u8, 134u8,
                            106u8, 76u8,
                        ],
                    )
                }
                #[doc = " The amount of balance a caller has to pay for each byte of storage."]
                #[doc = ""]
                #[doc = " # Note"]
                #[doc = ""]
                #[doc = " Changing this value for an existing chain might need a storage migration."]
                pub fn deposit_per_byte(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Contracts",
                        "DepositPerByte",
                        [
                            84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
                            27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
                            136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
                        ],
                    )
                }
                #[doc = " The amount of balance a caller has to pay for each storage item."]
                #[doc = ""]
                #[doc = " # Note"]
                #[doc = ""]
                #[doc = " Changing this value for an existing chain might need a storage migration."]
                pub fn deposit_per_item(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Contracts",
                        "DepositPerItem",
                        [
                            84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
                            27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
                            136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
                        ],
                    )
                }
                #[doc = " The maximum length of a contract code in bytes. This limit applies to the instrumented"]
                #[doc = " version of the code. Therefore `instantiate_with_code` can fail even when supplying"]
                #[doc = " a wasm binary below this maximum size."]
                #[doc = ""]
                #[doc = " The value should be chosen carefully taking into the account the overall memory limit"]
                #[doc = " your runtime has, as well as the [maximum allowed callstack"]
                #[doc = " depth](#associatedtype.CallStack). Look into the `integrity_test()` for some insights."]
                pub fn max_code_len(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Contracts",
                        "MaxCodeLen",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                #[doc = " The maximum allowable length in bytes for storage keys."]
                pub fn max_storage_key_len(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Contracts",
                        "MaxStorageKeyLen",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                #[doc = " Make contract callable functions marked as `#[unstable]` available."]
                #[doc = ""]
                #[doc = " Contracts that use `#[unstable]` functions won't be able to be uploaded unless"]
                #[doc = " this is set to `true`. This is only meant for testnets and dev nodes in order to"]
                #[doc = " experiment with new features."]
                #[doc = ""]
                #[doc = " # Warning"]
                #[doc = ""]
                #[doc = " Do **not** set to `true` on productions chains."]
                pub fn unsafe_unstable_interface(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::bool>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Contracts",
                        "UnsafeUnstableInterface",
                        [
                            165u8, 28u8, 112u8, 190u8, 18u8, 129u8, 182u8, 206u8, 237u8, 1u8, 68u8,
                            252u8, 125u8, 234u8, 185u8, 50u8, 149u8, 164u8, 47u8, 126u8, 134u8,
                            100u8, 14u8, 86u8, 209u8, 39u8, 20u8, 4u8, 233u8, 115u8, 102u8, 131u8,
                        ],
                    )
                }
                #[doc = " The maximum length of the debug buffer in bytes."]
                pub fn max_debug_buffer_len(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Contracts",
                        "MaxDebugBufferLen",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod nomination_pools {
        use super::{root_mod, runtime_types};
        #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Join {
                #[codec(compact)]
                pub amount: ::core::primitive::u128,
                pub pool_id: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct BondExtra {
                pub extra:
                    runtime_types::pallet_nomination_pools::BondExtra<::core::primitive::u128>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ClaimPayout;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Unbond {
                pub member_account: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                #[codec(compact)]
                pub unbonding_points: ::core::primitive::u128,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct PoolWithdrawUnbonded {
                pub pool_id: ::core::primitive::u32,
                pub num_slashing_spans: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct WithdrawUnbonded {
                pub member_account: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                pub num_slashing_spans: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Create {
                #[codec(compact)]
                pub amount: ::core::primitive::u128,
                pub root: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                pub nominator: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                pub state_toggler: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct CreateWithPoolId {
                #[codec(compact)]
                pub amount: ::core::primitive::u128,
                pub root: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                pub nominator: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                pub state_toggler: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                pub pool_id: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Nominate {
                pub pool_id: ::core::primitive::u32,
                pub validators: ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetState {
                pub pool_id: ::core::primitive::u32,
                pub state: runtime_types::pallet_nomination_pools::PoolState,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetMetadata {
                pub pool_id: ::core::primitive::u32,
                pub metadata: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetConfigs {
                pub min_join_bond:
                    runtime_types::pallet_nomination_pools::ConfigOp<::core::primitive::u128>,
                pub min_create_bond:
                    runtime_types::pallet_nomination_pools::ConfigOp<::core::primitive::u128>,
                pub max_pools:
                    runtime_types::pallet_nomination_pools::ConfigOp<::core::primitive::u32>,
                pub max_members:
                    runtime_types::pallet_nomination_pools::ConfigOp<::core::primitive::u32>,
                pub max_members_per_pool:
                    runtime_types::pallet_nomination_pools::ConfigOp<::core::primitive::u32>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct UpdateRoles {
                pub pool_id: ::core::primitive::u32,
                pub new_root: runtime_types::pallet_nomination_pools::ConfigOp<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                >,
                pub new_nominator: runtime_types::pallet_nomination_pools::ConfigOp<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                >,
                pub new_state_toggler: runtime_types::pallet_nomination_pools::ConfigOp<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: CompactAs,
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Chill {
                pub pool_id: ::core::primitive::u32,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                #[doc = "Stake funds with a pool. The amount to bond is transferred from the member to the"]
                #[doc = "pools account and immediately increases the pools bond."]
                #[doc = ""]
                #[doc = "# Note"]
                #[doc = ""]
                #[doc = "* An account can only be a member of a single pool."]
                #[doc = "* An account cannot join the same pool multiple times."]
                #[doc = "* This call will *not* dust the member account, so the member must have at least"]
                #[doc = "  `existential deposit + amount` in their account."]
                #[doc = "* Only a pool with [`PoolState::Open`] can be joined"]
                pub fn join(
                    &self,
                    amount: ::core::primitive::u128,
                    pool_id: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<Join> {
                    ::subxt::tx::StaticTxPayload::new(
                        "NominationPools",
                        "join",
                        Join { amount, pool_id },
                        [
                            205u8, 66u8, 42u8, 72u8, 146u8, 148u8, 119u8, 162u8, 101u8, 183u8,
                            46u8, 176u8, 221u8, 204u8, 197u8, 20u8, 75u8, 226u8, 29u8, 118u8,
                            208u8, 60u8, 192u8, 247u8, 222u8, 100u8, 69u8, 80u8, 172u8, 13u8, 69u8,
                            250u8,
                        ],
                    )
                }
                #[doc = "Bond `extra` more funds from `origin` into the pool to which they already belong."]
                #[doc = ""]
                #[doc = "Additional funds can come from either the free balance of the account, of from the"]
                #[doc = "accumulated rewards, see [`BondExtra`]."]
                #[doc = ""]
                #[doc = "Bonding extra funds implies an automatic payout of all pending rewards as well."]
                pub fn bond_extra(
                    &self,
                    extra: runtime_types::pallet_nomination_pools::BondExtra<
                        ::core::primitive::u128,
                    >,
                ) -> ::subxt::tx::StaticTxPayload<BondExtra> {
                    ::subxt::tx::StaticTxPayload::new(
                        "NominationPools",
                        "bond_extra",
                        BondExtra { extra },
                        [
                            50u8, 72u8, 181u8, 216u8, 249u8, 27u8, 250u8, 177u8, 253u8, 22u8,
                            240u8, 100u8, 184u8, 202u8, 197u8, 34u8, 21u8, 188u8, 248u8, 191u8,
                            11u8, 10u8, 236u8, 161u8, 168u8, 37u8, 38u8, 238u8, 61u8, 183u8, 86u8,
                            55u8,
                        ],
                    )
                }
                #[doc = "A bonded member can use this to claim their payout based on the rewards that the pool"]
                #[doc = "has accumulated since their last claimed payout (OR since joining if this is there first"]
                #[doc = "time claiming rewards). The payout will be transferred to the member's account."]
                #[doc = ""]
                #[doc = "The member will earn rewards pro rata based on the members stake vs the sum of the"]
                #[doc = "members in the pools stake. Rewards do not \"expire\"."]
                pub fn claim_payout(&self) -> ::subxt::tx::StaticTxPayload<ClaimPayout> {
                    ::subxt::tx::StaticTxPayload::new(
                        "NominationPools",
                        "claim_payout",
                        ClaimPayout {},
                        [
                            128u8, 58u8, 138u8, 55u8, 64u8, 16u8, 129u8, 25u8, 211u8, 229u8, 193u8,
                            115u8, 47u8, 45u8, 155u8, 221u8, 218u8, 1u8, 222u8, 5u8, 236u8, 32u8,
                            88u8, 0u8, 198u8, 72u8, 196u8, 181u8, 104u8, 16u8, 212u8, 29u8,
                        ],
                    )
                }
                #[doc = "Unbond up to `unbonding_points` of the `member_account`'s funds from the pool. It"]
                #[doc = "implicitly collects the rewards one last time, since not doing so would mean some"]
                #[doc = "rewards would be forfeited."]
                #[doc = ""]
                #[doc = "Under certain conditions, this call can be dispatched permissionlessly (i.e. by any"]
                #[doc = "account)."]
                #[doc = ""]
                #[doc = "# Conditions for a permissionless dispatch."]
                #[doc = ""]
                #[doc = "* The pool is blocked and the caller is either the root or state-toggler. This is"]
                #[doc = "  refereed to as a kick."]
                #[doc = "* The pool is destroying and the member is not the depositor."]
                #[doc = "* The pool is destroying, the member is the depositor and no other members are in the"]
                #[doc = "  pool."]
                #[doc = ""]
                #[doc = "## Conditions for permissioned dispatch (i.e. the caller is also the"]
                #[doc = "`member_account`):"]
                #[doc = ""]
                #[doc = "* The caller is not the depositor."]
                #[doc = "* The caller is the depositor, the pool is destroying and no other members are in the"]
                #[doc = "  pool."]
                #[doc = ""]
                #[doc = "# Note"]
                #[doc = ""]
                #[doc = "If there are too many unlocking chunks to unbond with the pool account,"]
                #[doc = "[`Call::pool_withdraw_unbonded`] can be called to try and minimize unlocking chunks."]
                #[doc = "The [`StakingInterface::unbond`] will implicitly call [`Call::pool_withdraw_unbonded`]"]
                #[doc = "to try to free chunks if necessary (ie. if unbound was called and no unlocking chunks"]
                #[doc = "are available). However, it may not be possible to release the current unlocking chunks,"]
                #[doc = "in which case, the result of this call will likely be the `NoMoreChunks` error from the"]
                #[doc = "staking system."]
                pub fn unbond(
                    &self,
                    member_account: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    unbonding_points: ::core::primitive::u128,
                ) -> ::subxt::tx::StaticTxPayload<Unbond> {
                    ::subxt::tx::StaticTxPayload::new(
                        "NominationPools",
                        "unbond",
                        Unbond {
                            member_account,
                            unbonding_points,
                        },
                        [
                            78u8, 15u8, 37u8, 18u8, 129u8, 63u8, 31u8, 3u8, 68u8, 10u8, 12u8, 12u8,
                            166u8, 179u8, 38u8, 232u8, 97u8, 1u8, 83u8, 53u8, 26u8, 59u8, 42u8,
                            219u8, 176u8, 246u8, 169u8, 28u8, 35u8, 67u8, 139u8, 81u8,
                        ],
                    )
                }
                #[doc = "Call `withdraw_unbonded` for the pools account. This call can be made by any account."]
                #[doc = ""]
                #[doc = "This is useful if their are too many unlocking chunks to call `unbond`, and some"]
                #[doc = "can be cleared by withdrawing. In the case there are too many unlocking chunks, the user"]
                #[doc = "would probably see an error like `NoMoreChunks` emitted from the staking system when"]
                #[doc = "they attempt to unbond."]
                pub fn pool_withdraw_unbonded(
                    &self,
                    pool_id: ::core::primitive::u32,
                    num_slashing_spans: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<PoolWithdrawUnbonded> {
                    ::subxt::tx::StaticTxPayload::new(
                        "NominationPools",
                        "pool_withdraw_unbonded",
                        PoolWithdrawUnbonded {
                            pool_id,
                            num_slashing_spans,
                        },
                        [
                            152u8, 245u8, 131u8, 247u8, 106u8, 214u8, 154u8, 8u8, 7u8, 210u8,
                            149u8, 218u8, 118u8, 46u8, 242u8, 182u8, 191u8, 119u8, 28u8, 199u8,
                            36u8, 49u8, 219u8, 123u8, 58u8, 203u8, 211u8, 226u8, 217u8, 36u8, 56u8,
                            0u8,
                        ],
                    )
                }
                #[doc = "Withdraw unbonded funds from `member_account`. If no bonded funds can be unbonded, an"]
                #[doc = "error is returned."]
                #[doc = ""]
                #[doc = "Under certain conditions, this call can be dispatched permissionlessly (i.e. by any"]
                #[doc = "account)."]
                #[doc = ""]
                #[doc = "# Conditions for a permissionless dispatch"]
                #[doc = ""]
                #[doc = "* The pool is in destroy mode and the target is not the depositor."]
                #[doc = "* The target is the depositor and they are the only member in the sub pools."]
                #[doc = "* The pool is blocked and the caller is either the root or state-toggler."]
                #[doc = ""]
                #[doc = "# Conditions for permissioned dispatch"]
                #[doc = ""]
                #[doc = "* The caller is the target and they are not the depositor."]
                #[doc = ""]
                #[doc = "# Note"]
                #[doc = ""]
                #[doc = "If the target is the depositor, the pool will be destroyed."]
                pub fn withdraw_unbonded(
                    &self,
                    member_account: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    num_slashing_spans: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<WithdrawUnbonded> {
                    ::subxt::tx::StaticTxPayload::new(
                        "NominationPools",
                        "withdraw_unbonded",
                        WithdrawUnbonded {
                            member_account,
                            num_slashing_spans,
                        },
                        [
                            61u8, 216u8, 214u8, 166u8, 59u8, 42u8, 186u8, 141u8, 47u8, 50u8, 135u8,
                            236u8, 166u8, 88u8, 90u8, 244u8, 57u8, 106u8, 193u8, 211u8, 215u8,
                            131u8, 203u8, 33u8, 195u8, 120u8, 213u8, 94u8, 213u8, 66u8, 79u8,
                            140u8,
                        ],
                    )
                }
                #[doc = "Create a new delegation pool."]
                #[doc = ""]
                #[doc = "# Arguments"]
                #[doc = ""]
                #[doc = "* `amount` - The amount of funds to delegate to the pool. This also acts of a sort of"]
                #[doc = "  deposit since the pools creator cannot fully unbond funds until the pool is being"]
                #[doc = "  destroyed."]
                #[doc = "* `index` - A disambiguation index for creating the account. Likely only useful when"]
                #[doc = "  creating multiple pools in the same extrinsic."]
                #[doc = "* `root` - The account to set as [`PoolRoles::root`]."]
                #[doc = "* `nominator` - The account to set as the [`PoolRoles::nominator`]."]
                #[doc = "* `state_toggler` - The account to set as the [`PoolRoles::state_toggler`]."]
                #[doc = ""]
                #[doc = "# Note"]
                #[doc = ""]
                #[doc = "In addition to `amount`, the caller will transfer the existential deposit; so the caller"]
                #[doc = "needs at have at least `amount + existential_deposit` transferrable."]
                pub fn create(
                    &self,
                    amount: ::core::primitive::u128,
                    root: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    nominator: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    state_toggler: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                ) -> ::subxt::tx::StaticTxPayload<Create> {
                    ::subxt::tx::StaticTxPayload::new(
                        "NominationPools",
                        "create",
                        Create {
                            amount,
                            root,
                            nominator,
                            state_toggler,
                        },
                        [
                            176u8, 210u8, 154u8, 87u8, 218u8, 250u8, 117u8, 90u8, 80u8, 191u8,
                            252u8, 146u8, 29u8, 228u8, 36u8, 15u8, 125u8, 102u8, 87u8, 50u8, 146u8,
                            108u8, 96u8, 145u8, 135u8, 189u8, 18u8, 159u8, 21u8, 74u8, 165u8, 33u8,
                        ],
                    )
                }
                #[doc = "Create a new delegation pool with a previously used pool id"]
                #[doc = ""]
                #[doc = "# Arguments"]
                #[doc = ""]
                #[doc = "same as `create` with the inclusion of"]
                #[doc = "* `pool_id` - `A valid PoolId."]
                pub fn create_with_pool_id(
                    &self,
                    amount: ::core::primitive::u128,
                    root: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    nominator: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    state_toggler: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    pool_id: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<CreateWithPoolId> {
                    ::subxt::tx::StaticTxPayload::new(
                        "NominationPools",
                        "create_with_pool_id",
                        CreateWithPoolId {
                            amount,
                            root,
                            nominator,
                            state_toggler,
                            pool_id,
                        },
                        [
                            234u8, 228u8, 116u8, 171u8, 77u8, 41u8, 166u8, 254u8, 20u8, 78u8, 38u8,
                            28u8, 144u8, 58u8, 2u8, 64u8, 11u8, 27u8, 124u8, 215u8, 8u8, 10u8,
                            172u8, 189u8, 118u8, 131u8, 102u8, 191u8, 251u8, 208u8, 167u8, 103u8,
                        ],
                    )
                }
                #[doc = "Nominate on behalf of the pool."]
                #[doc = ""]
                #[doc = "The dispatch origin of this call must be signed by the pool nominator or the pool"]
                #[doc = "root role."]
                #[doc = ""]
                #[doc = "This directly forward the call to the staking pallet, on behalf of the pool bonded"]
                #[doc = "account."]
                pub fn nominate(
                    &self,
                    pool_id: ::core::primitive::u32,
                    validators: ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::tx::StaticTxPayload<Nominate> {
                    ::subxt::tx::StaticTxPayload::new(
                        "NominationPools",
                        "nominate",
                        Nominate {
                            pool_id,
                            validators,
                        },
                        [
                            10u8, 235u8, 64u8, 157u8, 36u8, 249u8, 186u8, 27u8, 79u8, 172u8, 25u8,
                            3u8, 203u8, 19u8, 192u8, 182u8, 36u8, 103u8, 13u8, 20u8, 89u8, 140u8,
                            159u8, 4u8, 132u8, 242u8, 192u8, 146u8, 55u8, 251u8, 216u8, 255u8,
                        ],
                    )
                }
                #[doc = "Set a new state for the pool."]
                #[doc = ""]
                #[doc = "If a pool is already in the `Destroying` state, then under no condition can its state"]
                #[doc = "change again."]
                #[doc = ""]
                #[doc = "The dispatch origin of this call must be either:"]
                #[doc = ""]
                #[doc = "1. signed by the state toggler, or the root role of the pool,"]
                #[doc = "2. if the pool conditions to be open are NOT met (as described by `ok_to_be_open`), and"]
                #[doc = "   then the state of the pool can be permissionlessly changed to `Destroying`."]
                pub fn set_state(
                    &self,
                    pool_id: ::core::primitive::u32,
                    state: runtime_types::pallet_nomination_pools::PoolState,
                ) -> ::subxt::tx::StaticTxPayload<SetState> {
                    ::subxt::tx::StaticTxPayload::new(
                        "NominationPools",
                        "set_state",
                        SetState { pool_id, state },
                        [
                            104u8, 40u8, 213u8, 88u8, 159u8, 115u8, 35u8, 249u8, 78u8, 180u8, 99u8,
                            1u8, 225u8, 218u8, 192u8, 151u8, 25u8, 194u8, 192u8, 187u8, 39u8,
                            170u8, 212u8, 125u8, 75u8, 250u8, 248u8, 175u8, 159u8, 161u8, 151u8,
                            162u8,
                        ],
                    )
                }
                #[doc = "Set a new metadata for the pool."]
                #[doc = ""]
                #[doc = "The dispatch origin of this call must be signed by the state toggler, or the root role"]
                #[doc = "of the pool."]
                pub fn set_metadata(
                    &self,
                    pool_id: ::core::primitive::u32,
                    metadata: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::StaticTxPayload<SetMetadata> {
                    ::subxt::tx::StaticTxPayload::new(
                        "NominationPools",
                        "set_metadata",
                        SetMetadata { pool_id, metadata },
                        [
                            156u8, 81u8, 170u8, 161u8, 34u8, 100u8, 183u8, 174u8, 5u8, 81u8, 31u8,
                            76u8, 12u8, 42u8, 77u8, 1u8, 6u8, 26u8, 168u8, 7u8, 8u8, 115u8, 158u8,
                            151u8, 30u8, 211u8, 52u8, 177u8, 234u8, 87u8, 125u8, 127u8,
                        ],
                    )
                }
                #[doc = "Update configurations for the nomination pools. The origin for this call must be"]
                #[doc = "Root."]
                #[doc = ""]
                #[doc = "# Arguments"]
                #[doc = ""]
                #[doc = "* `min_join_bond` - Set [`MinJoinBond`]."]
                #[doc = "* `min_create_bond` - Set [`MinCreateBond`]."]
                #[doc = "* `max_pools` - Set [`MaxPools`]."]
                #[doc = "* `max_members` - Set [`MaxPoolMembers`]."]
                #[doc = "* `max_members_per_pool` - Set [`MaxPoolMembersPerPool`]."]
                pub fn set_configs(
                    &self,
                    min_join_bond: runtime_types::pallet_nomination_pools::ConfigOp<
                        ::core::primitive::u128,
                    >,
                    min_create_bond: runtime_types::pallet_nomination_pools::ConfigOp<
                        ::core::primitive::u128,
                    >,
                    max_pools: runtime_types::pallet_nomination_pools::ConfigOp<
                        ::core::primitive::u32,
                    >,
                    max_members: runtime_types::pallet_nomination_pools::ConfigOp<
                        ::core::primitive::u32,
                    >,
                    max_members_per_pool: runtime_types::pallet_nomination_pools::ConfigOp<
                        ::core::primitive::u32,
                    >,
                ) -> ::subxt::tx::StaticTxPayload<SetConfigs> {
                    ::subxt::tx::StaticTxPayload::new(
                        "NominationPools",
                        "set_configs",
                        SetConfigs {
                            min_join_bond,
                            min_create_bond,
                            max_pools,
                            max_members,
                            max_members_per_pool,
                        },
                        [
                            143u8, 196u8, 211u8, 30u8, 71u8, 15u8, 150u8, 243u8, 7u8, 178u8, 179u8,
                            168u8, 40u8, 116u8, 220u8, 140u8, 18u8, 206u8, 6u8, 189u8, 190u8, 37u8,
                            68u8, 41u8, 45u8, 233u8, 247u8, 172u8, 185u8, 34u8, 243u8, 187u8,
                        ],
                    )
                }
                #[doc = "Update the roles of the pool."]
                #[doc = ""]
                #[doc = "The root is the only entity that can change any of the roles, including itself,"]
                #[doc = "excluding the depositor, who can never change."]
                #[doc = ""]
                #[doc = "It emits an event, notifying UIs of the role change. This event is quite relevant to"]
                #[doc = "most pool members and they should be informed of changes to pool roles."]
                pub fn update_roles(
                    &self,
                    pool_id: ::core::primitive::u32,
                    new_root: runtime_types::pallet_nomination_pools::ConfigOp<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                    >,
                    new_nominator: runtime_types::pallet_nomination_pools::ConfigOp<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                    >,
                    new_state_toggler: runtime_types::pallet_nomination_pools::ConfigOp<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                    >,
                ) -> ::subxt::tx::StaticTxPayload<UpdateRoles> {
                    ::subxt::tx::StaticTxPayload::new(
                        "NominationPools",
                        "update_roles",
                        UpdateRoles {
                            pool_id,
                            new_root,
                            new_nominator,
                            new_state_toggler,
                        },
                        [
                            247u8, 95u8, 234u8, 56u8, 181u8, 229u8, 158u8, 97u8, 69u8, 165u8, 38u8,
                            17u8, 27u8, 209u8, 204u8, 250u8, 91u8, 193u8, 35u8, 93u8, 215u8, 131u8,
                            148u8, 73u8, 67u8, 188u8, 92u8, 32u8, 34u8, 37u8, 113u8, 93u8,
                        ],
                    )
                }
                #[doc = "Chill on behalf of the pool."]
                #[doc = ""]
                #[doc = "The dispatch origin of this call must be signed by the pool nominator or the pool"]
                #[doc = "root role, same as [`Pallet::nominate`]."]
                #[doc = ""]
                #[doc = "This directly forward the call to the staking pallet, on behalf of the pool bonded"]
                #[doc = "account."]
                pub fn chill(
                    &self,
                    pool_id: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<Chill> {
                    ::subxt::tx::StaticTxPayload::new(
                        "NominationPools",
                        "chill",
                        Chill { pool_id },
                        [
                            41u8, 114u8, 128u8, 121u8, 244u8, 15u8, 15u8, 52u8, 129u8, 88u8, 239u8,
                            167u8, 216u8, 38u8, 123u8, 240u8, 172u8, 229u8, 132u8, 64u8, 175u8,
                            87u8, 217u8, 27u8, 11u8, 124u8, 1u8, 140u8, 40u8, 191u8, 187u8, 36u8,
                        ],
                    )
                }
            }
        }
        #[doc = "Events of this pallet."]
        pub type Event = runtime_types::pallet_nomination_pools::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A pool has been created."]
            pub struct Created {
                pub depositor: ::subxt::ext::sp_core::crypto::AccountId32,
                pub pool_id: ::core::primitive::u32,
            }
            impl ::subxt::events::StaticEvent for Created {
                const PALLET: &'static str = "NominationPools";
                const EVENT: &'static str = "Created";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A member has became bonded in a pool."]
            pub struct Bonded {
                pub member: ::subxt::ext::sp_core::crypto::AccountId32,
                pub pool_id: ::core::primitive::u32,
                pub bonded: ::core::primitive::u128,
                pub joined: ::core::primitive::bool,
            }
            impl ::subxt::events::StaticEvent for Bonded {
                const PALLET: &'static str = "NominationPools";
                const EVENT: &'static str = "Bonded";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A payout has been made to a member."]
            pub struct PaidOut {
                pub member: ::subxt::ext::sp_core::crypto::AccountId32,
                pub pool_id: ::core::primitive::u32,
                pub payout: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for PaidOut {
                const PALLET: &'static str = "NominationPools";
                const EVENT: &'static str = "PaidOut";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A member has unbonded from their pool."]
            #[doc = ""]
            #[doc = "- `balance` is the corresponding balance of the number of points that has been"]
            #[doc = "  requested to be unbonded (the argument of the `unbond` transaction) from the bonded"]
            #[doc = "  pool."]
            #[doc = "- `points` is the number of points that are issued as a result of `balance` being"]
            #[doc = "dissolved into the corresponding unbonding pool."]
            #[doc = "- `era` is the era in which the balance will be unbonded."]
            #[doc = "In the absence of slashing, these values will match. In the presence of slashing, the"]
            #[doc = "number of points that are issued in the unbonding pool will be less than the amount"]
            #[doc = "requested to be unbonded."]
            pub struct Unbonded {
                pub member: ::subxt::ext::sp_core::crypto::AccountId32,
                pub pool_id: ::core::primitive::u32,
                pub balance: ::core::primitive::u128,
                pub points: ::core::primitive::u128,
                pub era: ::core::primitive::u32,
            }
            impl ::subxt::events::StaticEvent for Unbonded {
                const PALLET: &'static str = "NominationPools";
                const EVENT: &'static str = "Unbonded";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A member has withdrawn from their pool."]
            #[doc = ""]
            #[doc = "The given number of `points` have been dissolved in return of `balance`."]
            #[doc = ""]
            #[doc = "Similar to `Unbonded` event, in the absence of slashing, the ratio of point to balance"]
            #[doc = "will be 1."]
            pub struct Withdrawn {
                pub member: ::subxt::ext::sp_core::crypto::AccountId32,
                pub pool_id: ::core::primitive::u32,
                pub balance: ::core::primitive::u128,
                pub points: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Withdrawn {
                const PALLET: &'static str = "NominationPools";
                const EVENT: &'static str = "Withdrawn";
            }
            #[derive(
                :: subxt :: ext :: codec :: CompactAs,
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A pool has been destroyed."]
            pub struct Destroyed {
                pub pool_id: ::core::primitive::u32,
            }
            impl ::subxt::events::StaticEvent for Destroyed {
                const PALLET: &'static str = "NominationPools";
                const EVENT: &'static str = "Destroyed";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "The state of a pool has changed"]
            pub struct StateChanged {
                pub pool_id: ::core::primitive::u32,
                pub new_state: runtime_types::pallet_nomination_pools::PoolState,
            }
            impl ::subxt::events::StaticEvent for StateChanged {
                const PALLET: &'static str = "NominationPools";
                const EVENT: &'static str = "StateChanged";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A member has been removed from a pool."]
            #[doc = ""]
            #[doc = "The removal can be voluntary (withdrawn all unbonded funds) or involuntary (kicked)."]
            pub struct MemberRemoved {
                pub pool_id: ::core::primitive::u32,
                pub member: ::subxt::ext::sp_core::crypto::AccountId32,
            }
            impl ::subxt::events::StaticEvent for MemberRemoved {
                const PALLET: &'static str = "NominationPools";
                const EVENT: &'static str = "MemberRemoved";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "The roles of a pool have been updated to the given new roles. Note that the depositor"]
            #[doc = "can never change."]
            pub struct RolesUpdated {
                pub root: ::core::option::Option<::subxt::ext::sp_core::crypto::AccountId32>,
                pub state_toggler:
                    ::core::option::Option<::subxt::ext::sp_core::crypto::AccountId32>,
                pub nominator: ::core::option::Option<::subxt::ext::sp_core::crypto::AccountId32>,
            }
            impl ::subxt::events::StaticEvent for RolesUpdated {
                const PALLET: &'static str = "NominationPools";
                const EVENT: &'static str = "RolesUpdated";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "The active balance of pool `pool_id` has been slashed to `balance`."]
            pub struct PoolSlashed {
                pub pool_id: ::core::primitive::u32,
                pub balance: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for PoolSlashed {
                const PALLET: &'static str = "NominationPools";
                const EVENT: &'static str = "PoolSlashed";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "The unbond pool at `era` of pool `pool_id` has been slashed to `balance`."]
            pub struct UnbondingPoolSlashed {
                pub pool_id: ::core::primitive::u32,
                pub era: ::core::primitive::u32,
                pub balance: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for UnbondingPoolSlashed {
                const PALLET: &'static str = "NominationPools";
                const EVENT: &'static str = "UnbondingPoolSlashed";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                #[doc = " Minimum amount to bond to join a pool."]
                pub fn min_join_bond(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "MinJoinBond",
                        vec![],
                        [
                            125u8, 239u8, 45u8, 225u8, 74u8, 129u8, 247u8, 184u8, 205u8, 58u8,
                            45u8, 186u8, 126u8, 170u8, 112u8, 120u8, 23u8, 190u8, 247u8, 97u8,
                            131u8, 126u8, 215u8, 44u8, 147u8, 122u8, 132u8, 212u8, 217u8, 84u8,
                            240u8, 91u8,
                        ],
                    )
                }
                #[doc = " Minimum bond required to create a pool."]
                #[doc = ""]
                #[doc = " This is the amount that the depositor must put as their initial stake in the pool, as an"]
                #[doc = " indication of \"skin in the game\"."]
                #[doc = ""]
                #[doc = " This is the value that will always exist in the staking ledger of the pool bonded account"]
                #[doc = " while all other accounts leave."]
                pub fn min_create_bond(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "MinCreateBond",
                        vec![],
                        [
                            31u8, 208u8, 240u8, 158u8, 23u8, 218u8, 212u8, 138u8, 92u8, 210u8,
                            207u8, 170u8, 32u8, 60u8, 5u8, 21u8, 84u8, 162u8, 1u8, 111u8, 181u8,
                            243u8, 24u8, 148u8, 193u8, 253u8, 248u8, 190u8, 16u8, 222u8, 219u8,
                            67u8,
                        ],
                    )
                }
                #[doc = " Maximum number of nomination pools that can exist. If `None`, then an unbounded number of"]
                #[doc = " pools can exist."]
                pub fn max_pools(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "MaxPools",
                        vec![],
                        [
                            216u8, 111u8, 68u8, 103u8, 33u8, 50u8, 109u8, 3u8, 176u8, 195u8, 23u8,
                            73u8, 112u8, 138u8, 9u8, 194u8, 233u8, 73u8, 68u8, 215u8, 162u8, 255u8,
                            217u8, 173u8, 141u8, 27u8, 72u8, 199u8, 7u8, 240u8, 25u8, 34u8,
                        ],
                    )
                }
                #[doc = " Maximum number of members that can exist in the system. If `None`, then the count"]
                #[doc = " members are not bound on a system wide basis."]
                pub fn max_pool_members(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "MaxPoolMembers",
                        vec![],
                        [
                            82u8, 217u8, 26u8, 234u8, 223u8, 241u8, 66u8, 182u8, 43u8, 233u8, 59u8,
                            242u8, 202u8, 254u8, 69u8, 50u8, 254u8, 196u8, 166u8, 89u8, 120u8,
                            87u8, 76u8, 148u8, 31u8, 197u8, 49u8, 88u8, 206u8, 41u8, 242u8, 62u8,
                        ],
                    )
                }
                #[doc = " Maximum number of members that may belong to pool. If `None`, then the count of"]
                #[doc = " members is not bound on a per pool basis."]
                pub fn max_pool_members_per_pool(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "MaxPoolMembersPerPool",
                        vec![],
                        [
                            93u8, 241u8, 16u8, 169u8, 138u8, 199u8, 128u8, 149u8, 65u8, 30u8, 55u8,
                            11u8, 41u8, 252u8, 83u8, 250u8, 9u8, 33u8, 152u8, 239u8, 195u8, 147u8,
                            16u8, 248u8, 180u8, 153u8, 88u8, 231u8, 248u8, 169u8, 186u8, 48u8,
                        ],
                    )
                }
                #[doc = " Active members."]
                #[doc = ""]
                #[doc = " TWOX-NOTE: SAFE since `AccountId` is a secure hash."]
                pub fn pool_members(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_nomination_pools::PoolMember,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "PoolMembers",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            252u8, 236u8, 201u8, 127u8, 219u8, 1u8, 19u8, 144u8, 5u8, 108u8, 70u8,
                            30u8, 177u8, 232u8, 253u8, 237u8, 211u8, 91u8, 63u8, 62u8, 155u8,
                            151u8, 153u8, 165u8, 206u8, 53u8, 111u8, 31u8, 60u8, 120u8, 100u8,
                            249u8,
                        ],
                    )
                }
                #[doc = " Active members."]
                #[doc = ""]
                #[doc = " TWOX-NOTE: SAFE since `AccountId` is a secure hash."]
                pub fn pool_members_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_nomination_pools::PoolMember,
                    >,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "PoolMembers",
                        Vec::new(),
                        [
                            252u8, 236u8, 201u8, 127u8, 219u8, 1u8, 19u8, 144u8, 5u8, 108u8, 70u8,
                            30u8, 177u8, 232u8, 253u8, 237u8, 211u8, 91u8, 63u8, 62u8, 155u8,
                            151u8, 153u8, 165u8, 206u8, 53u8, 111u8, 31u8, 60u8, 120u8, 100u8,
                            249u8,
                        ],
                    )
                }
                #[doc = "Counter for the related counted storage map"]
                pub fn counter_for_pool_members(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "CounterForPoolMembers",
                        vec![],
                        [
                            114u8, 126u8, 27u8, 138u8, 119u8, 44u8, 45u8, 129u8, 84u8, 107u8,
                            171u8, 206u8, 117u8, 141u8, 20u8, 75u8, 229u8, 237u8, 31u8, 229u8,
                            124u8, 190u8, 27u8, 124u8, 63u8, 59u8, 167u8, 42u8, 62u8, 212u8, 160u8,
                            2u8,
                        ],
                    )
                }
                #[doc = " Storage for bonded pools."]
                pub fn bonded_pools(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_nomination_pools::BondedPoolInner,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "BondedPools",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            34u8, 51u8, 86u8, 95u8, 237u8, 118u8, 40u8, 212u8, 128u8, 227u8, 113u8,
                            6u8, 116u8, 28u8, 96u8, 223u8, 63u8, 249u8, 33u8, 152u8, 61u8, 7u8,
                            205u8, 220u8, 221u8, 174u8, 207u8, 39u8, 53u8, 176u8, 13u8, 74u8,
                        ],
                    )
                }
                #[doc = " Storage for bonded pools."]
                pub fn bonded_pools_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_nomination_pools::BondedPoolInner,
                    >,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "BondedPools",
                        Vec::new(),
                        [
                            34u8, 51u8, 86u8, 95u8, 237u8, 118u8, 40u8, 212u8, 128u8, 227u8, 113u8,
                            6u8, 116u8, 28u8, 96u8, 223u8, 63u8, 249u8, 33u8, 152u8, 61u8, 7u8,
                            205u8, 220u8, 221u8, 174u8, 207u8, 39u8, 53u8, 176u8, 13u8, 74u8,
                        ],
                    )
                }
                #[doc = "Counter for the related counted storage map"]
                pub fn counter_for_bonded_pools(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "CounterForBondedPools",
                        vec![],
                        [
                            134u8, 94u8, 199u8, 73u8, 174u8, 253u8, 66u8, 242u8, 233u8, 244u8,
                            140u8, 170u8, 242u8, 40u8, 41u8, 185u8, 183u8, 151u8, 58u8, 111u8,
                            221u8, 225u8, 81u8, 71u8, 169u8, 219u8, 223u8, 135u8, 8u8, 171u8,
                            180u8, 236u8,
                        ],
                    )
                }
                #[doc = " Reward pools. This is where there rewards for each pool accumulate. When a members payout"]
                #[doc = " is claimed, the balance comes out fo the reward pool. Keyed by the bonded pools account."]
                pub fn reward_pools(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_nomination_pools::RewardPool,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "RewardPools",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            139u8, 123u8, 46u8, 107u8, 9u8, 83u8, 141u8, 12u8, 188u8, 225u8, 170u8,
                            215u8, 154u8, 21u8, 100u8, 95u8, 237u8, 245u8, 46u8, 216u8, 199u8,
                            184u8, 187u8, 155u8, 8u8, 16u8, 34u8, 177u8, 153u8, 65u8, 109u8, 198u8,
                        ],
                    )
                }
                #[doc = " Reward pools. This is where there rewards for each pool accumulate. When a members payout"]
                #[doc = " is claimed, the balance comes out fo the reward pool. Keyed by the bonded pools account."]
                pub fn reward_pools_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_nomination_pools::RewardPool,
                    >,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "RewardPools",
                        Vec::new(),
                        [
                            139u8, 123u8, 46u8, 107u8, 9u8, 83u8, 141u8, 12u8, 188u8, 225u8, 170u8,
                            215u8, 154u8, 21u8, 100u8, 95u8, 237u8, 245u8, 46u8, 216u8, 199u8,
                            184u8, 187u8, 155u8, 8u8, 16u8, 34u8, 177u8, 153u8, 65u8, 109u8, 198u8,
                        ],
                    )
                }
                #[doc = "Counter for the related counted storage map"]
                pub fn counter_for_reward_pools(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "CounterForRewardPools",
                        vec![],
                        [
                            209u8, 139u8, 212u8, 116u8, 210u8, 178u8, 213u8, 38u8, 75u8, 23u8,
                            188u8, 57u8, 253u8, 213u8, 95u8, 118u8, 182u8, 250u8, 45u8, 205u8,
                            17u8, 175u8, 17u8, 201u8, 234u8, 14u8, 98u8, 49u8, 143u8, 135u8, 201u8,
                            81u8,
                        ],
                    )
                }
                #[doc = " Groups of unbonding pools. Each group of unbonding pools belongs to a bonded pool,"]
                #[doc = " hence the name sub-pools. Keyed by the bonded pools account."]
                pub fn sub_pools_storage(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_nomination_pools::SubPools,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "SubPoolsStorage",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            231u8, 13u8, 111u8, 248u8, 1u8, 208u8, 179u8, 134u8, 224u8, 196u8,
                            94u8, 201u8, 229u8, 29u8, 155u8, 211u8, 163u8, 150u8, 157u8, 34u8,
                            68u8, 238u8, 55u8, 4u8, 222u8, 96u8, 186u8, 29u8, 205u8, 237u8, 80u8,
                            42u8,
                        ],
                    )
                }
                #[doc = " Groups of unbonding pools. Each group of unbonding pools belongs to a bonded pool,"]
                #[doc = " hence the name sub-pools. Keyed by the bonded pools account."]
                pub fn sub_pools_storage_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_nomination_pools::SubPools,
                    >,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "SubPoolsStorage",
                        Vec::new(),
                        [
                            231u8, 13u8, 111u8, 248u8, 1u8, 208u8, 179u8, 134u8, 224u8, 196u8,
                            94u8, 201u8, 229u8, 29u8, 155u8, 211u8, 163u8, 150u8, 157u8, 34u8,
                            68u8, 238u8, 55u8, 4u8, 222u8, 96u8, 186u8, 29u8, 205u8, 237u8, 80u8,
                            42u8,
                        ],
                    )
                }
                #[doc = "Counter for the related counted storage map"]
                pub fn counter_for_sub_pools_storage(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "CounterForSubPoolsStorage",
                        vec![],
                        [
                            212u8, 145u8, 212u8, 226u8, 234u8, 31u8, 26u8, 240u8, 107u8, 91u8,
                            171u8, 120u8, 41u8, 195u8, 16u8, 86u8, 55u8, 127u8, 103u8, 93u8, 128u8,
                            48u8, 69u8, 104u8, 168u8, 236u8, 81u8, 54u8, 2u8, 184u8, 215u8, 51u8,
                        ],
                    )
                }
                #[doc = " Metadata for the pool."]
                pub fn metadata(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            ::core::primitive::u8,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "Metadata",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            108u8, 250u8, 163u8, 54u8, 192u8, 143u8, 239u8, 62u8, 97u8, 163u8,
                            161u8, 215u8, 171u8, 225u8, 49u8, 18u8, 37u8, 200u8, 143u8, 254u8,
                            136u8, 26u8, 54u8, 187u8, 39u8, 3u8, 216u8, 24u8, 188u8, 25u8, 243u8,
                            251u8,
                        ],
                    )
                }
                #[doc = " Metadata for the pool."]
                pub fn metadata_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            ::core::primitive::u8,
                        >,
                    >,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "Metadata",
                        Vec::new(),
                        [
                            108u8, 250u8, 163u8, 54u8, 192u8, 143u8, 239u8, 62u8, 97u8, 163u8,
                            161u8, 215u8, 171u8, 225u8, 49u8, 18u8, 37u8, 200u8, 143u8, 254u8,
                            136u8, 26u8, 54u8, 187u8, 39u8, 3u8, 216u8, 24u8, 188u8, 25u8, 243u8,
                            251u8,
                        ],
                    )
                }
                #[doc = "Counter for the related counted storage map"]
                pub fn counter_for_metadata(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "CounterForMetadata",
                        vec![],
                        [
                            190u8, 232u8, 77u8, 134u8, 245u8, 89u8, 160u8, 187u8, 163u8, 68u8,
                            188u8, 204u8, 31u8, 145u8, 219u8, 165u8, 213u8, 1u8, 167u8, 90u8,
                            175u8, 218u8, 147u8, 144u8, 158u8, 226u8, 23u8, 233u8, 55u8, 168u8,
                            161u8, 237u8,
                        ],
                    )
                }
                #[doc = " Ever increasing number of all pools created so far."]
                pub fn last_pool_id(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "LastPoolId",
                        vec![],
                        [
                            50u8, 254u8, 218u8, 41u8, 213u8, 184u8, 170u8, 166u8, 31u8, 29u8,
                            196u8, 57u8, 215u8, 20u8, 40u8, 40u8, 19u8, 22u8, 9u8, 184u8, 11u8,
                            21u8, 21u8, 125u8, 97u8, 38u8, 219u8, 209u8, 2u8, 238u8, 247u8, 51u8,
                        ],
                    )
                }
                #[doc = " A reverse lookup from the pool's account id to its id."]
                #[doc = ""]
                #[doc = " This is only used for slashing. In all other instances, the pool id is used, and the"]
                #[doc = " accounts are deterministically derived from it."]
                pub fn reverse_pool_id_lookup(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "ReversePoolIdLookup",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            178u8, 161u8, 51u8, 220u8, 128u8, 1u8, 135u8, 83u8, 236u8, 159u8, 36u8,
                            237u8, 120u8, 128u8, 6u8, 191u8, 41u8, 159u8, 94u8, 178u8, 174u8,
                            235u8, 221u8, 173u8, 44u8, 81u8, 211u8, 255u8, 231u8, 81u8, 16u8, 87u8,
                        ],
                    )
                }
                #[doc = " A reverse lookup from the pool's account id to its id."]
                #[doc = ""]
                #[doc = " This is only used for slashing. In all other instances, the pool id is used, and the"]
                #[doc = " accounts are deterministically derived from it."]
                pub fn reverse_pool_id_lookup_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "ReversePoolIdLookup",
                        Vec::new(),
                        [
                            178u8, 161u8, 51u8, 220u8, 128u8, 1u8, 135u8, 83u8, 236u8, 159u8, 36u8,
                            237u8, 120u8, 128u8, 6u8, 191u8, 41u8, 159u8, 94u8, 178u8, 174u8,
                            235u8, 221u8, 173u8, 44u8, 81u8, 211u8, 255u8, 231u8, 81u8, 16u8, 87u8,
                        ],
                    )
                }
                #[doc = "Counter for the related counted storage map"]
                pub fn counter_for_reverse_pool_id_lookup(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "NominationPools",
                        "CounterForReversePoolIdLookup",
                        vec![],
                        [
                            148u8, 83u8, 81u8, 33u8, 188u8, 72u8, 148u8, 208u8, 245u8, 178u8, 52u8,
                            245u8, 229u8, 140u8, 100u8, 152u8, 8u8, 217u8, 161u8, 80u8, 226u8,
                            42u8, 15u8, 252u8, 90u8, 197u8, 120u8, 114u8, 144u8, 90u8, 199u8,
                            123u8,
                        ],
                    )
                }
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                #[doc = " The nomination pool's pallet id."]
                pub fn pallet_id(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<runtime_types::frame_support::PalletId>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "NominationPools",
                        "PalletId",
                        [
                            139u8, 109u8, 228u8, 151u8, 252u8, 32u8, 130u8, 69u8, 112u8, 154u8,
                            174u8, 45u8, 83u8, 245u8, 51u8, 132u8, 173u8, 5u8, 186u8, 24u8, 243u8,
                            9u8, 12u8, 214u8, 80u8, 74u8, 69u8, 189u8, 30u8, 94u8, 22u8, 39u8,
                        ],
                    )
                }
                #[doc = " The maximum pool points-to-balance ratio that an `open` pool can have."]
                #[doc = ""]
                #[doc = " This is important in the event slashing takes place and the pool's points-to-balance"]
                #[doc = " ratio becomes disproportional."]
                #[doc = ""]
                #[doc = " Moreover, this relates to the `RewardCounter` type as well, as the arithmetic operations"]
                #[doc = " are a function of number of points, and by setting this value to e.g. 10, you ensure"]
                #[doc = " that the total number of points in the system are at most 10 times the total_issuance of"]
                #[doc = " the chain, in the absolute worse case."]
                #[doc = ""]
                #[doc = " For a value of 10, the threshold would be a pool points-to-balance ratio of 10:1."]
                #[doc = " Such a scenario would also be the equivalent of the pool being 90% slashed."]
                pub fn max_points_to_balance(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u8>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "NominationPools",
                        "MaxPointsToBalance",
                        [
                            141u8, 130u8, 11u8, 35u8, 226u8, 114u8, 92u8, 179u8, 168u8, 110u8,
                            28u8, 91u8, 221u8, 64u8, 4u8, 148u8, 201u8, 193u8, 185u8, 66u8, 226u8,
                            114u8, 97u8, 79u8, 62u8, 212u8, 202u8, 114u8, 237u8, 228u8, 183u8,
                            165u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod identity {
        use super::{root_mod, runtime_types};
        #[doc = "Identity pallet declaration."]
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct AddRegistrar {
                pub account: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetIdentity {
                pub info: ::std::boxed::Box<runtime_types::pallet_identity::types::IdentityInfo>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetSubs {
                pub subs: ::std::vec::Vec<(
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    runtime_types::pallet_identity::types::Data,
                )>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ClearIdentity;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct RequestJudgement {
                #[codec(compact)]
                pub reg_index: ::core::primitive::u32,
                #[codec(compact)]
                pub max_fee: ::core::primitive::u128,
            }
            #[derive(
                :: subxt :: ext :: codec :: CompactAs,
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct CancelRequest {
                pub reg_index: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetFee {
                #[codec(compact)]
                pub index: ::core::primitive::u32,
                #[codec(compact)]
                pub fee: ::core::primitive::u128,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetAccountId {
                #[codec(compact)]
                pub index: ::core::primitive::u32,
                pub new: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SetFields {
                #[codec(compact)]
                pub index: ::core::primitive::u32,
                pub fields: runtime_types::pallet_identity::types::BitFlags<
                    runtime_types::pallet_identity::types::IdentityField,
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ProvideJudgement {
                #[codec(compact)]
                pub reg_index: ::core::primitive::u32,
                pub target: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                pub judgement:
                    runtime_types::pallet_identity::types::Judgement<::core::primitive::u128>,
                pub identity: ::subxt::ext::sp_core::H256,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct KillIdentity {
                pub target: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct AddSub {
                pub sub: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                pub data: runtime_types::pallet_identity::types::Data,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct RenameSub {
                pub sub: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
                pub data: runtime_types::pallet_identity::types::Data,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct RemoveSub {
                pub sub: ::subxt::ext::sp_runtime::MultiAddress<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                    (),
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct QuitSub;
            pub struct TransactionApi;
            impl TransactionApi {
                #[doc = "Add a registrar to the system."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be `T::RegistrarOrigin`."]
                #[doc = ""]
                #[doc = "- `account`: the account of the registrar."]
                #[doc = ""]
                #[doc = "Emits `RegistrarAdded` if successful."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(R)` where `R` registrar-count (governance-bounded and code-bounded)."]
                #[doc = "- One storage mutation (codec `O(R)`)."]
                #[doc = "- One event."]
                #[doc = "# </weight>"]
                pub fn add_registrar(
                    &self,
                    account: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                ) -> ::subxt::tx::StaticTxPayload<AddRegistrar> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Identity",
                        "add_registrar",
                        AddRegistrar { account },
                        [
                            157u8, 232u8, 252u8, 190u8, 203u8, 233u8, 127u8, 63u8, 111u8, 16u8,
                            118u8, 200u8, 31u8, 234u8, 144u8, 111u8, 161u8, 224u8, 217u8, 86u8,
                            179u8, 254u8, 162u8, 212u8, 248u8, 8u8, 125u8, 89u8, 23u8, 195u8, 4u8,
                            231u8,
                        ],
                    )
                }
                #[doc = "Set an account's identity information and reserve the appropriate deposit."]
                #[doc = ""]
                #[doc = "If the account already has identity information, the deposit is taken as part payment"]
                #[doc = "for the new deposit."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_."]
                #[doc = ""]
                #[doc = "- `info`: The identity information."]
                #[doc = ""]
                #[doc = "Emits `IdentitySet` if successful."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(X + X' + R)`"]
                #[doc = "  - where `X` additional-field-count (deposit-bounded and code-bounded)"]
                #[doc = "  - where `R` judgements-count (registrar-count-bounded)"]
                #[doc = "- One balance reserve operation."]
                #[doc = "- One storage mutation (codec-read `O(X' + R)`, codec-write `O(X + R)`)."]
                #[doc = "- One event."]
                #[doc = "# </weight>"]
                pub fn set_identity(
                    &self,
                    info: runtime_types::pallet_identity::types::IdentityInfo,
                ) -> ::subxt::tx::StaticTxPayload<SetIdentity> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Identity",
                        "set_identity",
                        SetIdentity {
                            info: ::std::boxed::Box::new(info),
                        },
                        [
                            130u8, 89u8, 118u8, 6u8, 134u8, 166u8, 35u8, 192u8, 73u8, 6u8, 171u8,
                            20u8, 225u8, 255u8, 152u8, 142u8, 111u8, 8u8, 206u8, 200u8, 64u8, 52u8,
                            110u8, 123u8, 42u8, 101u8, 191u8, 242u8, 133u8, 139u8, 154u8, 205u8,
                        ],
                    )
                }
                #[doc = "Set the sub-accounts of the sender."]
                #[doc = ""]
                #[doc = "Payment: Any aggregate balance reserved by previous `set_subs` calls will be returned"]
                #[doc = "and an amount `SubAccountDeposit` will be reserved for each item in `subs`."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ and the sender must have a registered"]
                #[doc = "identity."]
                #[doc = ""]
                #[doc = "- `subs`: The identity's (new) sub-accounts."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(P + S)`"]
                #[doc = "  - where `P` old-subs-count (hard- and deposit-bounded)."]
                #[doc = "  - where `S` subs-count (hard- and deposit-bounded)."]
                #[doc = "- At most one balance operations."]
                #[doc = "- DB:"]
                #[doc = "  - `P + S` storage mutations (codec complexity `O(1)`)"]
                #[doc = "  - One storage read (codec complexity `O(P)`)."]
                #[doc = "  - One storage write (codec complexity `O(S)`)."]
                #[doc = "  - One storage-exists (`IdentityOf::contains_key`)."]
                #[doc = "# </weight>"]
                pub fn set_subs(
                    &self,
                    subs: ::std::vec::Vec<(
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        runtime_types::pallet_identity::types::Data,
                    )>,
                ) -> ::subxt::tx::StaticTxPayload<SetSubs> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Identity",
                        "set_subs",
                        SetSubs { subs },
                        [
                            177u8, 219u8, 84u8, 183u8, 5u8, 32u8, 192u8, 82u8, 174u8, 68u8, 198u8,
                            224u8, 56u8, 85u8, 134u8, 171u8, 30u8, 132u8, 140u8, 236u8, 117u8,
                            24u8, 150u8, 218u8, 146u8, 194u8, 144u8, 92u8, 103u8, 206u8, 46u8,
                            90u8,
                        ],
                    )
                }
                #[doc = "Clear an account's identity info and all sub-accounts and return all deposits."]
                #[doc = ""]
                #[doc = "Payment: All reserved balances on the account are returned."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ and the sender must have a registered"]
                #[doc = "identity."]
                #[doc = ""]
                #[doc = "Emits `IdentityCleared` if successful."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(R + S + X)`"]
                #[doc = "  - where `R` registrar-count (governance-bounded)."]
                #[doc = "  - where `S` subs-count (hard- and deposit-bounded)."]
                #[doc = "  - where `X` additional-field-count (deposit-bounded and code-bounded)."]
                #[doc = "- One balance-unreserve operation."]
                #[doc = "- `2` storage reads and `S + 2` storage deletions."]
                #[doc = "- One event."]
                #[doc = "# </weight>"]
                pub fn clear_identity(&self) -> ::subxt::tx::StaticTxPayload<ClearIdentity> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Identity",
                        "clear_identity",
                        ClearIdentity {},
                        [
                            75u8, 44u8, 74u8, 122u8, 149u8, 202u8, 114u8, 230u8, 0u8, 255u8, 140u8,
                            122u8, 14u8, 196u8, 205u8, 249u8, 220u8, 94u8, 216u8, 34u8, 63u8, 14u8,
                            8u8, 205u8, 74u8, 23u8, 181u8, 129u8, 252u8, 110u8, 231u8, 114u8,
                        ],
                    )
                }
                #[doc = "Request a judgement from a registrar."]
                #[doc = ""]
                #[doc = "Payment: At most `max_fee` will be reserved for payment to the registrar if judgement"]
                #[doc = "given."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ and the sender must have a"]
                #[doc = "registered identity."]
                #[doc = ""]
                #[doc = "- `reg_index`: The index of the registrar whose judgement is requested."]
                #[doc = "- `max_fee`: The maximum fee that may be paid. This should just be auto-populated as:"]
                #[doc = ""]
                #[doc = "```nocompile"]
                #[doc = "Self::registrars().get(reg_index).unwrap().fee"]
                #[doc = "```"]
                #[doc = ""]
                #[doc = "Emits `JudgementRequested` if successful."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(R + X)`."]
                #[doc = "- One balance-reserve operation."]
                #[doc = "- Storage: 1 read `O(R)`, 1 mutate `O(X + R)`."]
                #[doc = "- One event."]
                #[doc = "# </weight>"]
                pub fn request_judgement(
                    &self,
                    reg_index: ::core::primitive::u32,
                    max_fee: ::core::primitive::u128,
                ) -> ::subxt::tx::StaticTxPayload<RequestJudgement> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Identity",
                        "request_judgement",
                        RequestJudgement { reg_index, max_fee },
                        [
                            186u8, 149u8, 61u8, 54u8, 159u8, 194u8, 77u8, 161u8, 220u8, 157u8, 3u8,
                            216u8, 23u8, 105u8, 119u8, 76u8, 144u8, 198u8, 157u8, 45u8, 235u8,
                            139u8, 87u8, 82u8, 81u8, 12u8, 25u8, 134u8, 225u8, 92u8, 182u8, 101u8,
                        ],
                    )
                }
                #[doc = "Cancel a previous request."]
                #[doc = ""]
                #[doc = "Payment: A previously reserved deposit is returned on success."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ and the sender must have a"]
                #[doc = "registered identity."]
                #[doc = ""]
                #[doc = "- `reg_index`: The index of the registrar whose judgement is no longer requested."]
                #[doc = ""]
                #[doc = "Emits `JudgementUnrequested` if successful."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(R + X)`."]
                #[doc = "- One balance-reserve operation."]
                #[doc = "- One storage mutation `O(R + X)`."]
                #[doc = "- One event"]
                #[doc = "# </weight>"]
                pub fn cancel_request(
                    &self,
                    reg_index: ::core::primitive::u32,
                ) -> ::subxt::tx::StaticTxPayload<CancelRequest> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Identity",
                        "cancel_request",
                        CancelRequest { reg_index },
                        [
                            83u8, 180u8, 239u8, 126u8, 32u8, 51u8, 17u8, 20u8, 180u8, 3u8, 59u8,
                            96u8, 24u8, 32u8, 136u8, 92u8, 58u8, 254u8, 68u8, 70u8, 50u8, 11u8,
                            51u8, 91u8, 180u8, 79u8, 81u8, 84u8, 216u8, 138u8, 6u8, 215u8,
                        ],
                    )
                }
                #[doc = "Set the fee required for a judgement to be requested from a registrar."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ and the sender must be the account"]
                #[doc = "of the registrar whose index is `index`."]
                #[doc = ""]
                #[doc = "- `index`: the index of the registrar whose fee is to be set."]
                #[doc = "- `fee`: the new fee."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(R)`."]
                #[doc = "- One storage mutation `O(R)`."]
                #[doc = "- Benchmark: 7.315 + R * 0.329 µs (min squares analysis)"]
                #[doc = "# </weight>"]
                pub fn set_fee(
                    &self,
                    index: ::core::primitive::u32,
                    fee: ::core::primitive::u128,
                ) -> ::subxt::tx::StaticTxPayload<SetFee> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Identity",
                        "set_fee",
                        SetFee { index, fee },
                        [
                            21u8, 157u8, 123u8, 182u8, 160u8, 190u8, 117u8, 37u8, 136u8, 133u8,
                            104u8, 234u8, 31u8, 145u8, 115u8, 154u8, 125u8, 40u8, 2u8, 87u8, 118u8,
                            56u8, 247u8, 73u8, 89u8, 0u8, 251u8, 3u8, 58u8, 105u8, 239u8, 211u8,
                        ],
                    )
                }
                #[doc = "Change the account associated with a registrar."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ and the sender must be the account"]
                #[doc = "of the registrar whose index is `index`."]
                #[doc = ""]
                #[doc = "- `index`: the index of the registrar whose fee is to be set."]
                #[doc = "- `new`: the new account ID."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(R)`."]
                #[doc = "- One storage mutation `O(R)`."]
                #[doc = "- Benchmark: 8.823 + R * 0.32 µs (min squares analysis)"]
                #[doc = "# </weight>"]
                pub fn set_account_id(
                    &self,
                    index: ::core::primitive::u32,
                    new: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                ) -> ::subxt::tx::StaticTxPayload<SetAccountId> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Identity",
                        "set_account_id",
                        SetAccountId { index, new },
                        [
                            13u8, 91u8, 36u8, 7u8, 88u8, 64u8, 151u8, 104u8, 94u8, 174u8, 195u8,
                            99u8, 97u8, 181u8, 236u8, 251u8, 26u8, 236u8, 234u8, 40u8, 183u8, 38u8,
                            220u8, 216u8, 48u8, 115u8, 7u8, 230u8, 216u8, 28u8, 123u8, 11u8,
                        ],
                    )
                }
                #[doc = "Set the field information for a registrar."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ and the sender must be the account"]
                #[doc = "of the registrar whose index is `index`."]
                #[doc = ""]
                #[doc = "- `index`: the index of the registrar whose fee is to be set."]
                #[doc = "- `fields`: the fields that the registrar concerns themselves with."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(R)`."]
                #[doc = "- One storage mutation `O(R)`."]
                #[doc = "- Benchmark: 7.464 + R * 0.325 µs (min squares analysis)"]
                #[doc = "# </weight>"]
                pub fn set_fields(
                    &self,
                    index: ::core::primitive::u32,
                    fields: runtime_types::pallet_identity::types::BitFlags<
                        runtime_types::pallet_identity::types::IdentityField,
                    >,
                ) -> ::subxt::tx::StaticTxPayload<SetFields> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Identity",
                        "set_fields",
                        SetFields { index, fields },
                        [
                            50u8, 196u8, 179u8, 71u8, 66u8, 65u8, 235u8, 7u8, 51u8, 14u8, 81u8,
                            173u8, 201u8, 58u8, 6u8, 151u8, 174u8, 245u8, 102u8, 184u8, 28u8, 84u8,
                            125u8, 93u8, 126u8, 134u8, 92u8, 203u8, 200u8, 129u8, 240u8, 252u8,
                        ],
                    )
                }
                #[doc = "Provide a judgement for an account's identity."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ and the sender must be the account"]
                #[doc = "of the registrar whose index is `reg_index`."]
                #[doc = ""]
                #[doc = "- `reg_index`: the index of the registrar whose judgement is being made."]
                #[doc = "- `target`: the account whose identity the judgement is upon. This must be an account"]
                #[doc = "  with a registered identity."]
                #[doc = "- `judgement`: the judgement of the registrar of index `reg_index` about `target`."]
                #[doc = "- `identity`: The hash of the [`IdentityInfo`] for that the judgement is provided."]
                #[doc = ""]
                #[doc = "Emits `JudgementGiven` if successful."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(R + X)`."]
                #[doc = "- One balance-transfer operation."]
                #[doc = "- Up to one account-lookup operation."]
                #[doc = "- Storage: 1 read `O(R)`, 1 mutate `O(R + X)`."]
                #[doc = "- One event."]
                #[doc = "# </weight>"]
                pub fn provide_judgement(
                    &self,
                    reg_index: ::core::primitive::u32,
                    target: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    judgement: runtime_types::pallet_identity::types::Judgement<
                        ::core::primitive::u128,
                    >,
                    identity: ::subxt::ext::sp_core::H256,
                ) -> ::subxt::tx::StaticTxPayload<ProvideJudgement> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Identity",
                        "provide_judgement",
                        ProvideJudgement {
                            reg_index,
                            target,
                            judgement,
                            identity,
                        },
                        [
                            147u8, 66u8, 29u8, 90u8, 149u8, 65u8, 161u8, 115u8, 12u8, 254u8, 188u8,
                            248u8, 165u8, 115u8, 191u8, 2u8, 167u8, 223u8, 199u8, 169u8, 203u8,
                            64u8, 101u8, 217u8, 73u8, 185u8, 93u8, 109u8, 22u8, 184u8, 146u8, 73u8,
                        ],
                    )
                }
                #[doc = "Remove an account's identity and sub-account information and slash the deposits."]
                #[doc = ""]
                #[doc = "Payment: Reserved balances from `set_subs` and `set_identity` are slashed and handled by"]
                #[doc = "`Slash`. Verification request deposits are not returned; they should be cancelled"]
                #[doc = "manually using `cancel_request`."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must match `T::ForceOrigin`."]
                #[doc = ""]
                #[doc = "- `target`: the account whose identity the judgement is upon. This must be an account"]
                #[doc = "  with a registered identity."]
                #[doc = ""]
                #[doc = "Emits `IdentityKilled` if successful."]
                #[doc = ""]
                #[doc = "# <weight>"]
                #[doc = "- `O(R + S + X)`."]
                #[doc = "- One balance-reserve operation."]
                #[doc = "- `S + 2` storage mutations."]
                #[doc = "- One event."]
                #[doc = "# </weight>"]
                pub fn kill_identity(
                    &self,
                    target: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                ) -> ::subxt::tx::StaticTxPayload<KillIdentity> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Identity",
                        "kill_identity",
                        KillIdentity { target },
                        [
                            76u8, 13u8, 158u8, 219u8, 221u8, 0u8, 151u8, 241u8, 137u8, 136u8,
                            179u8, 194u8, 188u8, 230u8, 56u8, 16u8, 254u8, 28u8, 127u8, 216u8,
                            205u8, 117u8, 224u8, 121u8, 240u8, 231u8, 126u8, 181u8, 230u8, 68u8,
                            13u8, 174u8,
                        ],
                    )
                }
                #[doc = "Add the given account to the sender's subs."]
                #[doc = ""]
                #[doc = "Payment: Balance reserved by a previous `set_subs` call for one sub will be repatriated"]
                #[doc = "to the sender."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ and the sender must have a registered"]
                #[doc = "sub identity of `sub`."]
                pub fn add_sub(
                    &self,
                    sub: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    data: runtime_types::pallet_identity::types::Data,
                ) -> ::subxt::tx::StaticTxPayload<AddSub> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Identity",
                        "add_sub",
                        AddSub { sub, data },
                        [
                            122u8, 218u8, 25u8, 93u8, 33u8, 176u8, 191u8, 254u8, 223u8, 147u8,
                            100u8, 135u8, 86u8, 71u8, 47u8, 163u8, 105u8, 222u8, 162u8, 173u8,
                            207u8, 182u8, 130u8, 128u8, 214u8, 242u8, 101u8, 250u8, 242u8, 24u8,
                            17u8, 84u8,
                        ],
                    )
                }
                #[doc = "Alter the associated name of the given sub-account."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ and the sender must have a registered"]
                #[doc = "sub identity of `sub`."]
                pub fn rename_sub(
                    &self,
                    sub: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                    data: runtime_types::pallet_identity::types::Data,
                ) -> ::subxt::tx::StaticTxPayload<RenameSub> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Identity",
                        "rename_sub",
                        RenameSub { sub, data },
                        [
                            166u8, 167u8, 49u8, 114u8, 199u8, 168u8, 187u8, 221u8, 100u8, 85u8,
                            147u8, 211u8, 157u8, 31u8, 109u8, 135u8, 194u8, 135u8, 15u8, 89u8,
                            59u8, 57u8, 252u8, 163u8, 9u8, 138u8, 216u8, 189u8, 177u8, 42u8, 96u8,
                            34u8,
                        ],
                    )
                }
                #[doc = "Remove the given account from the sender's subs."]
                #[doc = ""]
                #[doc = "Payment: Balance reserved by a previous `set_subs` call for one sub will be repatriated"]
                #[doc = "to the sender."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ and the sender must have a registered"]
                #[doc = "sub identity of `sub`."]
                pub fn remove_sub(
                    &self,
                    sub: ::subxt::ext::sp_runtime::MultiAddress<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        (),
                    >,
                ) -> ::subxt::tx::StaticTxPayload<RemoveSub> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Identity",
                        "remove_sub",
                        RemoveSub { sub },
                        [
                            106u8, 223u8, 210u8, 67u8, 54u8, 11u8, 144u8, 222u8, 42u8, 46u8, 157u8,
                            33u8, 13u8, 245u8, 166u8, 195u8, 227u8, 81u8, 224u8, 149u8, 154u8,
                            158u8, 187u8, 203u8, 215u8, 91u8, 43u8, 105u8, 69u8, 213u8, 141u8,
                            124u8,
                        ],
                    )
                }
                #[doc = "Remove the sender as a sub-account."]
                #[doc = ""]
                #[doc = "Payment: Balance reserved by a previous `set_subs` call for one sub will be repatriated"]
                #[doc = "to the sender (*not* the original depositor)."]
                #[doc = ""]
                #[doc = "The dispatch origin for this call must be _Signed_ and the sender must have a registered"]
                #[doc = "super-identity."]
                #[doc = ""]
                #[doc = "NOTE: This should not normally be used, but is provided in the case that the non-"]
                #[doc = "controller of an account is maliciously registered as a sub-account."]
                pub fn quit_sub(&self) -> ::subxt::tx::StaticTxPayload<QuitSub> {
                    ::subxt::tx::StaticTxPayload::new(
                        "Identity",
                        "quit_sub",
                        QuitSub {},
                        [
                            62u8, 57u8, 73u8, 72u8, 119u8, 216u8, 250u8, 155u8, 57u8, 169u8, 157u8,
                            44u8, 87u8, 51u8, 63u8, 231u8, 77u8, 7u8, 0u8, 119u8, 244u8, 42u8,
                            179u8, 51u8, 254u8, 240u8, 55u8, 25u8, 142u8, 38u8, 87u8, 44u8,
                        ],
                    )
                }
            }
        }
        #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
        pub type Event = runtime_types::pallet_identity::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A name was set or reset (which will remove all judgements)."]
            pub struct IdentitySet {
                pub who: ::subxt::ext::sp_core::crypto::AccountId32,
            }
            impl ::subxt::events::StaticEvent for IdentitySet {
                const PALLET: &'static str = "Identity";
                const EVENT: &'static str = "IdentitySet";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A name was cleared, and the given balance returned."]
            pub struct IdentityCleared {
                pub who: ::subxt::ext::sp_core::crypto::AccountId32,
                pub deposit: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for IdentityCleared {
                const PALLET: &'static str = "Identity";
                const EVENT: &'static str = "IdentityCleared";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A name was removed and the given balance slashed."]
            pub struct IdentityKilled {
                pub who: ::subxt::ext::sp_core::crypto::AccountId32,
                pub deposit: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for IdentityKilled {
                const PALLET: &'static str = "Identity";
                const EVENT: &'static str = "IdentityKilled";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A judgement was asked from a registrar."]
            pub struct JudgementRequested {
                pub who: ::subxt::ext::sp_core::crypto::AccountId32,
                pub registrar_index: ::core::primitive::u32,
            }
            impl ::subxt::events::StaticEvent for JudgementRequested {
                const PALLET: &'static str = "Identity";
                const EVENT: &'static str = "JudgementRequested";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A judgement request was retracted."]
            pub struct JudgementUnrequested {
                pub who: ::subxt::ext::sp_core::crypto::AccountId32,
                pub registrar_index: ::core::primitive::u32,
            }
            impl ::subxt::events::StaticEvent for JudgementUnrequested {
                const PALLET: &'static str = "Identity";
                const EVENT: &'static str = "JudgementUnrequested";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A judgement was given by a registrar."]
            pub struct JudgementGiven {
                pub target: ::subxt::ext::sp_core::crypto::AccountId32,
                pub registrar_index: ::core::primitive::u32,
            }
            impl ::subxt::events::StaticEvent for JudgementGiven {
                const PALLET: &'static str = "Identity";
                const EVENT: &'static str = "JudgementGiven";
            }
            #[derive(
                :: subxt :: ext :: codec :: CompactAs,
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A registrar was added."]
            pub struct RegistrarAdded {
                pub registrar_index: ::core::primitive::u32,
            }
            impl ::subxt::events::StaticEvent for RegistrarAdded {
                const PALLET: &'static str = "Identity";
                const EVENT: &'static str = "RegistrarAdded";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A sub-identity was added to an identity and the deposit paid."]
            pub struct SubIdentityAdded {
                pub sub: ::subxt::ext::sp_core::crypto::AccountId32,
                pub main: ::subxt::ext::sp_core::crypto::AccountId32,
                pub deposit: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for SubIdentityAdded {
                const PALLET: &'static str = "Identity";
                const EVENT: &'static str = "SubIdentityAdded";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A sub-identity was removed from an identity and the deposit freed."]
            pub struct SubIdentityRemoved {
                pub sub: ::subxt::ext::sp_core::crypto::AccountId32,
                pub main: ::subxt::ext::sp_core::crypto::AccountId32,
                pub deposit: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for SubIdentityRemoved {
                const PALLET: &'static str = "Identity";
                const EVENT: &'static str = "SubIdentityRemoved";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "A sub-identity was cleared, and the given deposit repatriated from the"]
            #[doc = "main identity account to the sub-identity account."]
            pub struct SubIdentityRevoked {
                pub sub: ::subxt::ext::sp_core::crypto::AccountId32,
                pub main: ::subxt::ext::sp_core::crypto::AccountId32,
                pub deposit: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for SubIdentityRevoked {
                const PALLET: &'static str = "Identity";
                const EVENT: &'static str = "SubIdentityRevoked";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                #[doc = " Information that is pertinent to identify the entity behind an account."]
                #[doc = ""]
                #[doc = " TWOX-NOTE: OK ― `AccountId` is a secure hash."]
                pub fn identity_of(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_identity::types::Registration<
                            ::core::primitive::u128,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Identity",
                        "IdentityOf",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            193u8, 195u8, 180u8, 188u8, 129u8, 250u8, 180u8, 219u8, 22u8, 95u8,
                            175u8, 170u8, 143u8, 188u8, 80u8, 124u8, 234u8, 228u8, 245u8, 39u8,
                            72u8, 153u8, 107u8, 199u8, 23u8, 75u8, 47u8, 247u8, 104u8, 208u8,
                            171u8, 82u8,
                        ],
                    )
                }
                #[doc = " Information that is pertinent to identify the entity behind an account."]
                #[doc = ""]
                #[doc = " TWOX-NOTE: OK ― `AccountId` is a secure hash."]
                pub fn identity_of_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::pallet_identity::types::Registration<
                            ::core::primitive::u128,
                        >,
                    >,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Identity",
                        "IdentityOf",
                        Vec::new(),
                        [
                            193u8, 195u8, 180u8, 188u8, 129u8, 250u8, 180u8, 219u8, 22u8, 95u8,
                            175u8, 170u8, 143u8, 188u8, 80u8, 124u8, 234u8, 228u8, 245u8, 39u8,
                            72u8, 153u8, 107u8, 199u8, 23u8, 75u8, 47u8, 247u8, 104u8, 208u8,
                            171u8, 82u8,
                        ],
                    )
                }
                #[doc = " The super-identity of an alternative \"sub\" identity together with its name, within that"]
                #[doc = " context. If the account is not some other account's sub-identity, then just `None`."]
                pub fn super_of(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<(
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        runtime_types::pallet_identity::types::Data,
                    )>,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Identity",
                        "SuperOf",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Blake2_128Concat,
                        )],
                        [
                            170u8, 249u8, 112u8, 249u8, 75u8, 176u8, 21u8, 29u8, 152u8, 149u8,
                            69u8, 113u8, 20u8, 92u8, 113u8, 130u8, 135u8, 62u8, 18u8, 204u8, 166u8,
                            193u8, 133u8, 167u8, 248u8, 117u8, 80u8, 137u8, 158u8, 111u8, 100u8,
                            137u8,
                        ],
                    )
                }
                #[doc = " The super-identity of an alternative \"sub\" identity together with its name, within that"]
                #[doc = " context. If the account is not some other account's sub-identity, then just `None`."]
                pub fn super_of_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<(
                        ::subxt::ext::sp_core::crypto::AccountId32,
                        runtime_types::pallet_identity::types::Data,
                    )>,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Identity",
                        "SuperOf",
                        Vec::new(),
                        [
                            170u8, 249u8, 112u8, 249u8, 75u8, 176u8, 21u8, 29u8, 152u8, 149u8,
                            69u8, 113u8, 20u8, 92u8, 113u8, 130u8, 135u8, 62u8, 18u8, 204u8, 166u8,
                            193u8, 133u8, 167u8, 248u8, 117u8, 80u8, 137u8, 158u8, 111u8, 100u8,
                            137u8,
                        ],
                    )
                }
                #[doc = " Alternative \"sub\" identities of this account."]
                #[doc = ""]
                #[doc = " The first item is the deposit, the second is a vector of the accounts."]
                #[doc = ""]
                #[doc = " TWOX-NOTE: OK ― `AccountId` is a secure hash."]
                pub fn subs_of(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::ext::sp_core::crypto::AccountId32>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<(
                        ::core::primitive::u128,
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                        >,
                    )>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Identity",
                        "SubsOf",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            128u8, 15u8, 175u8, 155u8, 216u8, 225u8, 200u8, 169u8, 215u8, 206u8,
                            110u8, 22u8, 204u8, 89u8, 212u8, 210u8, 159u8, 169u8, 53u8, 7u8, 44u8,
                            164u8, 91u8, 151u8, 7u8, 227u8, 38u8, 230u8, 175u8, 84u8, 6u8, 4u8,
                        ],
                    )
                }
                #[doc = " Alternative \"sub\" identities of this account."]
                #[doc = ""]
                #[doc = " The first item is the deposit, the second is a vector of the accounts."]
                #[doc = ""]
                #[doc = " TWOX-NOTE: OK ― `AccountId` is a secure hash."]
                pub fn subs_of_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<(
                        ::core::primitive::u128,
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                        >,
                    )>,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Identity",
                        "SubsOf",
                        Vec::new(),
                        [
                            128u8, 15u8, 175u8, 155u8, 216u8, 225u8, 200u8, 169u8, 215u8, 206u8,
                            110u8, 22u8, 204u8, 89u8, 212u8, 210u8, 159u8, 169u8, 53u8, 7u8, 44u8,
                            164u8, 91u8, 151u8, 7u8, 227u8, 38u8, 230u8, 175u8, 84u8, 6u8, 4u8,
                        ],
                    )
                }
                #[doc = " The set of registrars. Not expected to get very big as can only be added through a"]
                #[doc = " special origin (likely a council motion)."]
                #[doc = ""]
                #[doc = " The index into this can be cast to `RegistrarIndex` to get a valid value."]
                pub fn registrars(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            ::core::option::Option<
                                runtime_types::pallet_identity::types::RegistrarInfo<
                                    ::core::primitive::u128,
                                    ::subxt::ext::sp_core::crypto::AccountId32,
                                >,
                            >,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "Identity",
                        "Registrars",
                        vec![],
                        [
                            157u8, 87u8, 39u8, 240u8, 154u8, 54u8, 241u8, 229u8, 76u8, 9u8, 62u8,
                            252u8, 40u8, 143u8, 186u8, 182u8, 233u8, 187u8, 251u8, 61u8, 236u8,
                            229u8, 19u8, 55u8, 42u8, 36u8, 82u8, 173u8, 215u8, 155u8, 229u8, 111u8,
                        ],
                    )
                }
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                #[doc = " The amount held on deposit for a registered identity"]
                pub fn basic_deposit(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Identity",
                        "BasicDeposit",
                        [
                            84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
                            27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
                            136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
                        ],
                    )
                }
                #[doc = " The amount held on deposit per additional field for a registered identity."]
                pub fn field_deposit(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Identity",
                        "FieldDeposit",
                        [
                            84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
                            27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
                            136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
                        ],
                    )
                }
                #[doc = " The amount held on deposit for a registered subaccount. This should account for the fact"]
                #[doc = " that one storage item's value will increase by the size of an account ID, and there will"]
                #[doc = " be another trie item whose value is the size of an account ID plus 32 bytes."]
                pub fn sub_account_deposit(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u128>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Identity",
                        "SubAccountDeposit",
                        [
                            84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
                            27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
                            136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
                        ],
                    )
                }
                #[doc = " The maximum number of sub-accounts allowed per identified account."]
                pub fn max_sub_accounts(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Identity",
                        "MaxSubAccounts",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                #[doc = " Maximum number of additional fields that may be stored in an ID. Needed to bound the I/O"]
                #[doc = " required to access an identity, but can be pretty high."]
                pub fn max_additional_fields(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Identity",
                        "MaxAdditionalFields",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                #[doc = " Maxmimum number of registrars allowed in the system. Needed to bound the complexity"]
                #[doc = " of, e.g., updating judgements."]
                pub fn max_registrars(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "Identity",
                        "MaxRegistrars",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod baby_liminal {
        use super::{root_mod, runtime_types};
        #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct StoreKey {
                pub identifier: [::core::primitive::u8; 4usize],
                pub key: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct DeleteKey {
                pub identifier: [::core::primitive::u8; 4usize],
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct OverwriteKey {
                pub identifier: [::core::primitive::u8; 4usize],
                pub key: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Verify {
                pub verification_key_identifier: [::core::primitive::u8; 4usize],
                pub proof: ::std::vec::Vec<::core::primitive::u8>,
                pub public_input: ::std::vec::Vec<::core::primitive::u8>,
                pub system: runtime_types::pallet_baby_liminal::systems::ProvingSystem,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                #[doc = "Stores `key` under `identifier` in `VerificationKeys` map."]
                #[doc = ""]
                #[doc = "Fails if:"]
                #[doc = "- `key.len()` is greater than `MaximumVerificationKeyLength`, or"]
                #[doc = "- `identifier` has been already used"]
                #[doc = ""]
                #[doc = "`key` can come from any proving system - there are no checks that verify it, in"]
                #[doc = "particular, `key` can contain just trash bytes."]
                pub fn store_key(
                    &self,
                    identifier: [::core::primitive::u8; 4usize],
                    key: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::StaticTxPayload<StoreKey> {
                    ::subxt::tx::StaticTxPayload::new(
                        "BabyLiminal",
                        "store_key",
                        StoreKey { identifier, key },
                        [
                            82u8, 244u8, 238u8, 30u8, 70u8, 12u8, 77u8, 111u8, 6u8, 219u8, 132u8,
                            67u8, 7u8, 86u8, 96u8, 78u8, 16u8, 94u8, 113u8, 141u8, 137u8, 159u8,
                            73u8, 199u8, 222u8, 59u8, 147u8, 146u8, 89u8, 254u8, 147u8, 71u8,
                        ],
                    )
                }
                #[doc = "Deletes a key stored under `identifier` in `VerificationKeys` map."]
                #[doc = ""]
                #[doc = "Can only be called by a root account."]
                pub fn delete_key(
                    &self,
                    identifier: [::core::primitive::u8; 4usize],
                ) -> ::subxt::tx::StaticTxPayload<DeleteKey> {
                    ::subxt::tx::StaticTxPayload::new(
                        "BabyLiminal",
                        "delete_key",
                        DeleteKey { identifier },
                        [
                            90u8, 90u8, 9u8, 204u8, 113u8, 9u8, 121u8, 115u8, 82u8, 197u8, 188u8,
                            78u8, 181u8, 133u8, 181u8, 87u8, 24u8, 207u8, 6u8, 156u8, 114u8, 218u8,
                            11u8, 97u8, 47u8, 252u8, 29u8, 101u8, 95u8, 208u8, 17u8, 95u8,
                        ],
                    )
                }
                #[doc = "Overwrites a key stored under `identifier` in `VerificationKeys` map with a new value `key`"]
                #[doc = ""]
                #[doc = "Fails if `key.len()` is greater than `MaximumVerificationKeyLength`."]
                #[doc = "Can only be called by a root account."]
                pub fn overwrite_key(
                    &self,
                    identifier: [::core::primitive::u8; 4usize],
                    key: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::StaticTxPayload<OverwriteKey> {
                    ::subxt::tx::StaticTxPayload::new(
                        "BabyLiminal",
                        "overwrite_key",
                        OverwriteKey { identifier, key },
                        [
                            187u8, 3u8, 92u8, 67u8, 8u8, 151u8, 35u8, 150u8, 161u8, 251u8, 238u8,
                            106u8, 50u8, 233u8, 177u8, 251u8, 111u8, 221u8, 33u8, 1u8, 30u8, 215u8,
                            69u8, 251u8, 249u8, 3u8, 139u8, 195u8, 4u8, 75u8, 86u8, 199u8,
                        ],
                    )
                }
                #[doc = "Verifies `proof` against `public_input` with a key that has been stored under"]
                #[doc = "`verification_key_identifier`. All is done within `system` proving system."]
                #[doc = ""]
                #[doc = "Fails if:"]
                #[doc = "- there is no verification key under `verification_key_identifier`"]
                #[doc = "- verification key under `verification_key_identifier` cannot be deserialized"]
                #[doc = "(e.g. it has been produced for another proving system)"]
                #[doc = "- `proof` cannot be deserialized (e.g. it has been produced for another proving system)"]
                #[doc = "- `public_input` cannot be deserialized (e.g. it has been produced for another proving"]
                #[doc = "system)"]
                #[doc = "- verifying procedure fails (e.g. incompatible verification key and proof)"]
                #[doc = "- proof is incorrect"]
                pub fn verify(
                    &self,
                    verification_key_identifier: [::core::primitive::u8; 4usize],
                    proof: ::std::vec::Vec<::core::primitive::u8>,
                    public_input: ::std::vec::Vec<::core::primitive::u8>,
                    system: runtime_types::pallet_baby_liminal::systems::ProvingSystem,
                ) -> ::subxt::tx::StaticTxPayload<Verify> {
                    ::subxt::tx::StaticTxPayload::new(
                        "BabyLiminal",
                        "verify",
                        Verify {
                            verification_key_identifier,
                            proof,
                            public_input,
                            system,
                        },
                        [
                            19u8, 255u8, 124u8, 213u8, 240u8, 67u8, 232u8, 85u8, 160u8, 172u8,
                            141u8, 161u8, 111u8, 92u8, 145u8, 8u8, 96u8, 2u8, 71u8, 112u8, 78u8,
                            123u8, 51u8, 120u8, 181u8, 103u8, 38u8, 49u8, 10u8, 6u8, 143u8, 218u8,
                        ],
                    )
                }
            }
        }
        #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
        pub type Event = runtime_types::pallet_baby_liminal::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Verification key has been successfully stored."]
            pub struct VerificationKeyStored;
            impl ::subxt::events::StaticEvent for VerificationKeyStored {
                const PALLET: &'static str = "BabyLiminal";
                const EVENT: &'static str = "VerificationKeyStored";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Verification key has been successfully deleted."]
            pub struct VerificationKeyDeleted;
            impl ::subxt::events::StaticEvent for VerificationKeyDeleted {
                const PALLET: &'static str = "BabyLiminal";
                const EVENT: &'static str = "VerificationKeyDeleted";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Verification key has been successfully overwritten."]
            pub struct VerificationKeyOverwritten;
            impl ::subxt::events::StaticEvent for VerificationKeyOverwritten {
                const PALLET: &'static str = "BabyLiminal";
                const EVENT: &'static str = "VerificationKeyOverwritten";
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            #[doc = "Proof has been successfully verified."]
            pub struct VerificationSucceeded;
            impl ::subxt::events::StaticEvent for VerificationSucceeded {
                const PALLET: &'static str = "BabyLiminal";
                const EVENT: &'static str = "VerificationSucceeded";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                pub fn verification_keys(
                    &self,
                    _0: impl ::std::borrow::Borrow<[::core::primitive::u8; 4usize]>,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            ::core::primitive::u8,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "BabyLiminal",
                        "VerificationKeys",
                        vec![::subxt::storage::address::StorageMapKey::new(
                            _0.borrow(),
                            ::subxt::storage::address::StorageHasher::Twox64Concat,
                        )],
                        [
                            138u8, 202u8, 229u8, 120u8, 126u8, 213u8, 192u8, 164u8, 44u8, 147u8,
                            126u8, 95u8, 210u8, 56u8, 118u8, 171u8, 85u8, 56u8, 75u8, 195u8, 233u8,
                            18u8, 246u8, 109u8, 114u8, 55u8, 181u8, 28u8, 42u8, 211u8, 83u8, 138u8,
                        ],
                    )
                }
                pub fn verification_keys_root(
                    &self,
                ) -> ::subxt::storage::address::StaticStorageAddress<
                    ::subxt::metadata::DecodeStaticType<
                        runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                            ::core::primitive::u8,
                        >,
                    >,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::StaticStorageAddress::new(
                        "BabyLiminal",
                        "VerificationKeys",
                        Vec::new(),
                        [
                            138u8, 202u8, 229u8, 120u8, 126u8, 213u8, 192u8, 164u8, 44u8, 147u8,
                            126u8, 95u8, 210u8, 56u8, 118u8, 171u8, 85u8, 56u8, 75u8, 195u8, 233u8,
                            18u8, 246u8, 109u8, 114u8, 55u8, 181u8, 28u8, 42u8, 211u8, 83u8, 138u8,
                        ],
                    )
                }
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                #[doc = " Limits how many bytes verification key can have."]
                #[doc = ""]
                #[doc = " Verification keys are stored, therefore this is separated from the limits on proof or"]
                #[doc = " public input."]
                pub fn maximum_verification_key_length(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "BabyLiminal",
                        "MaximumVerificationKeyLength",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                #[doc = " Limits how many bytes proof or public input can have."]
                pub fn maximum_data_length(
                    &self,
                ) -> ::subxt::constants::StaticConstantAddress<
                    ::subxt::metadata::DecodeStaticType<::core::primitive::u32>,
                > {
                    ::subxt::constants::StaticConstantAddress::new(
                        "BabyLiminal",
                        "MaximumDataLength",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod runtime_types {
        use super::runtime_types;
        pub mod aleph_runtime {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum OriginCaller {
                #[codec(index = 0)]
                system(
                    runtime_types::frame_support::dispatch::RawOrigin<
                        ::subxt::ext::sp_core::crypto::AccountId32,
                    >,
                ),
                #[codec(index = 1)]
                Void(runtime_types::sp_core::Void),
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Runtime;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum RuntimeCall {
                #[codec(index = 0)]
                System(runtime_types::frame_system::pallet::Call),
                #[codec(index = 2)]
                Scheduler(runtime_types::pallet_scheduler::pallet::Call),
                #[codec(index = 4)]
                Timestamp(runtime_types::pallet_timestamp::pallet::Call),
                #[codec(index = 5)]
                Balances(runtime_types::pallet_balances::pallet::Call),
                #[codec(index = 8)]
                Staking(runtime_types::pallet_staking::pallet::pallet::Call),
                #[codec(index = 10)]
                Session(runtime_types::pallet_session::pallet::Call),
                #[codec(index = 11)]
                Aleph(runtime_types::pallet_aleph::pallet::Call),
                #[codec(index = 12)]
                Elections(runtime_types::pallet_elections::pallet::Call),
                #[codec(index = 13)]
                Treasury(runtime_types::pallet_treasury::pallet::Call),
                #[codec(index = 14)]
                Vesting(runtime_types::pallet_vesting::pallet::Call),
                #[codec(index = 15)]
                Utility(runtime_types::pallet_utility::pallet::Call),
                #[codec(index = 16)]
                Multisig(runtime_types::pallet_multisig::pallet::Call),
                #[codec(index = 17)]
                Sudo(runtime_types::pallet_sudo::pallet::Call),
                #[codec(index = 18)]
                Contracts(runtime_types::pallet_contracts::pallet::Call),
                #[codec(index = 19)]
                NominationPools(runtime_types::pallet_nomination_pools::pallet::Call),
                #[codec(index = 20)]
                Identity(runtime_types::pallet_identity::pallet::Call),
                #[codec(index = 21)]
                BabyLiminal(runtime_types::pallet_baby_liminal::pallet::Call),
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum RuntimeEvent {
                #[codec(index = 0)]
                System(runtime_types::frame_system::pallet::Event),
                #[codec(index = 2)]
                Scheduler(runtime_types::pallet_scheduler::pallet::Event),
                #[codec(index = 5)]
                Balances(runtime_types::pallet_balances::pallet::Event),
                #[codec(index = 6)]
                TransactionPayment(runtime_types::pallet_transaction_payment::pallet::Event),
                #[codec(index = 8)]
                Staking(runtime_types::pallet_staking::pallet::pallet::Event),
                #[codec(index = 10)]
                Session(runtime_types::pallet_session::pallet::Event),
                #[codec(index = 11)]
                Aleph(runtime_types::pallet_aleph::pallet::Event),
                #[codec(index = 12)]
                Elections(runtime_types::pallet_elections::pallet::Event),
                #[codec(index = 13)]
                Treasury(runtime_types::pallet_treasury::pallet::Event),
                #[codec(index = 14)]
                Vesting(runtime_types::pallet_vesting::pallet::Event),
                #[codec(index = 15)]
                Utility(runtime_types::pallet_utility::pallet::Event),
                #[codec(index = 16)]
                Multisig(runtime_types::pallet_multisig::pallet::Event),
                #[codec(index = 17)]
                Sudo(runtime_types::pallet_sudo::pallet::Event),
                #[codec(index = 18)]
                Contracts(runtime_types::pallet_contracts::pallet::Event),
                #[codec(index = 19)]
                NominationPools(runtime_types::pallet_nomination_pools::pallet::Event),
                #[codec(index = 20)]
                Identity(runtime_types::pallet_identity::pallet::Event),
                #[codec(index = 21)]
                BabyLiminal(runtime_types::pallet_baby_liminal::pallet::Event),
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SessionKeys {
                pub aura: runtime_types::sp_consensus_aura::sr25519::app_sr25519::Public,
                pub aleph: runtime_types::primitives::app::Public,
            }
        }
        pub mod frame_support {
            use super::runtime_types;
            pub mod dispatch {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub enum DispatchClass {
                    #[codec(index = 0)]
                    Normal,
                    #[codec(index = 1)]
                    Operational,
                    #[codec(index = 2)]
                    Mandatory,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct DispatchInfo {
                    pub weight: runtime_types::sp_weights::weight_v2::Weight,
                    pub class: runtime_types::frame_support::dispatch::DispatchClass,
                    pub pays_fee: runtime_types::frame_support::dispatch::Pays,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub enum Pays {
                    #[codec(index = 0)]
                    Yes,
                    #[codec(index = 1)]
                    No,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct PerDispatchClass<_0> {
                    pub normal: _0,
                    pub operational: _0,
                    pub mandatory: _0,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub enum RawOrigin<_0> {
                    #[codec(index = 0)]
                    Root,
                    #[codec(index = 1)]
                    Signed(_0),
                    #[codec(index = 2)]
                    None,
                }
            }
            pub mod traits {
                use super::runtime_types;
                pub mod preimages {
                    use super::runtime_types;
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    pub enum Bounded<_0> {
                        #[codec(index = 0)]
                        Legacy {
                            hash: ::subxt::ext::sp_core::H256,
                        },
                        #[codec(index = 1)]
                        Inline(
                            runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                                ::core::primitive::u8,
                            >,
                        ),
                        #[codec(index = 2)]
                        Lookup {
                            hash: ::subxt::ext::sp_core::H256,
                            len: ::core::primitive::u32,
                        },
                        __Ignore(::core::marker::PhantomData<_0>),
                    }
                }
                pub mod tokens {
                    use super::runtime_types;
                    pub mod misc {
                        use super::runtime_types;
                        #[derive(
                            :: subxt :: ext :: codec :: Decode,
                            :: subxt :: ext :: codec :: Encode,
                            Clone,
                            Debug,
                            Eq,
                            PartialEq,
                        )]
                        pub enum BalanceStatus {
                            #[codec(index = 0)]
                            Free,
                            #[codec(index = 1)]
                            Reserved,
                        }
                    }
                }
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct PalletId(pub [::core::primitive::u8; 8usize]);
        }
        pub mod frame_system {
            use super::runtime_types;
            pub mod extensions {
                use super::runtime_types;
                pub mod check_genesis {
                    use super::runtime_types;
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    pub struct CheckGenesis;
                }
                pub mod check_mortality {
                    use super::runtime_types;
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    pub struct CheckMortality(pub runtime_types::sp_runtime::generic::era::Era);
                }
                pub mod check_nonce {
                    use super::runtime_types;
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    pub struct CheckNonce(#[codec(compact)] pub ::core::primitive::u32);
                }
                pub mod check_spec_version {
                    use super::runtime_types;
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    pub struct CheckSpecVersion;
                }
                pub mod check_tx_version {
                    use super::runtime_types;
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    pub struct CheckTxVersion;
                }
                pub mod check_weight {
                    use super::runtime_types;
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    pub struct CheckWeight;
                }
            }
            pub mod limits {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct BlockLength {
                    pub max: runtime_types::frame_support::dispatch::PerDispatchClass<
                        ::core::primitive::u32,
                    >,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct BlockWeights {
                    pub base_block: runtime_types::sp_weights::weight_v2::Weight,
                    pub max_block: runtime_types::sp_weights::weight_v2::Weight,
                    pub per_class: runtime_types::frame_support::dispatch::PerDispatchClass<
                        runtime_types::frame_system::limits::WeightsPerClass,
                    >,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct WeightsPerClass {
                    pub base_extrinsic: runtime_types::sp_weights::weight_v2::Weight,
                    pub max_extrinsic:
                        ::core::option::Option<runtime_types::sp_weights::weight_v2::Weight>,
                    pub max_total:
                        ::core::option::Option<runtime_types::sp_weights::weight_v2::Weight>,
                    pub reserved:
                        ::core::option::Option<runtime_types::sp_weights::weight_v2::Weight>,
                }
            }
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
                pub enum Call {
                    #[codec(index = 0)]
                    #[doc = "Make some on-chain remark."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(1)`"]
                    #[doc = "# </weight>"]
                    remark {
                        remark: ::std::vec::Vec<::core::primitive::u8>,
                    },
                    #[codec(index = 1)]
                    #[doc = "Set the number of pages in the WebAssembly environment's heap."]
                    set_heap_pages { pages: ::core::primitive::u64 },
                    #[codec(index = 2)]
                    #[doc = "Set the new runtime code."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(C + S)` where `C` length of `code` and `S` complexity of `can_set_code`"]
                    #[doc = "- 1 call to `can_set_code`: `O(S)` (calls `sp_io::misc::runtime_version` which is"]
                    #[doc = "  expensive)."]
                    #[doc = "- 1 storage write (codec `O(C)`)."]
                    #[doc = "- 1 digest item."]
                    #[doc = "- 1 event."]
                    #[doc = "The weight of this function is dependent on the runtime, but generally this is very"]
                    #[doc = "expensive. We will treat this as a full block."]
                    #[doc = "# </weight>"]
                    set_code {
                        code: ::std::vec::Vec<::core::primitive::u8>,
                    },
                    #[codec(index = 3)]
                    #[doc = "Set the new runtime code without doing any checks of the given `code`."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(C)` where `C` length of `code`"]
                    #[doc = "- 1 storage write (codec `O(C)`)."]
                    #[doc = "- 1 digest item."]
                    #[doc = "- 1 event."]
                    #[doc = "The weight of this function is dependent on the runtime. We will treat this as a full"]
                    #[doc = "block. # </weight>"]
                    set_code_without_checks {
                        code: ::std::vec::Vec<::core::primitive::u8>,
                    },
                    #[codec(index = 4)]
                    #[doc = "Set some items of storage."]
                    set_storage {
                        items: ::std::vec::Vec<(
                            ::std::vec::Vec<::core::primitive::u8>,
                            ::std::vec::Vec<::core::primitive::u8>,
                        )>,
                    },
                    #[codec(index = 5)]
                    #[doc = "Kill some items from storage."]
                    kill_storage {
                        keys: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
                    },
                    #[codec(index = 6)]
                    #[doc = "Kill all storage items with a key that starts with the given prefix."]
                    #[doc = ""]
                    #[doc = "**NOTE:** We rely on the Root origin to provide us the number of subkeys under"]
                    #[doc = "the prefix we are removing to accurately calculate the weight of this function."]
                    kill_prefix {
                        prefix: ::std::vec::Vec<::core::primitive::u8>,
                        subkeys: ::core::primitive::u32,
                    },
                    #[codec(index = 7)]
                    #[doc = "Make some on-chain remark and emit event."]
                    remark_with_event {
                        remark: ::std::vec::Vec<::core::primitive::u8>,
                    },
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Error for the System pallet"]
                pub enum Error {
                    #[codec(index = 0)]
                    #[doc = "The name of specification does not match between the current runtime"]
                    #[doc = "and the new runtime."]
                    InvalidSpecName,
                    #[codec(index = 1)]
                    #[doc = "The specification version is not allowed to decrease between the current runtime"]
                    #[doc = "and the new runtime."]
                    SpecVersionNeedsToIncrease,
                    #[codec(index = 2)]
                    #[doc = "Failed to extract the runtime version from the new runtime."]
                    #[doc = ""]
                    #[doc = "Either calling `Core_version` or decoding `RuntimeVersion` failed."]
                    FailedToExtractRuntimeVersion,
                    #[codec(index = 3)]
                    #[doc = "Suicide called when the account has non-default composite data."]
                    NonDefaultComposite,
                    #[codec(index = 4)]
                    #[doc = "There is a non-zero reference count preventing the account from being purged."]
                    NonZeroRefCount,
                    #[codec(index = 5)]
                    #[doc = "The origin filter prevent the call to be dispatched."]
                    CallFiltered,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Event for the System pallet."]
                pub enum Event {
                    #[codec(index = 0)]
                    #[doc = "An extrinsic completed successfully."]
                    ExtrinsicSuccess {
                        dispatch_info: runtime_types::frame_support::dispatch::DispatchInfo,
                    },
                    #[codec(index = 1)]
                    #[doc = "An extrinsic failed."]
                    ExtrinsicFailed {
                        dispatch_error: runtime_types::sp_runtime::DispatchError,
                        dispatch_info: runtime_types::frame_support::dispatch::DispatchInfo,
                    },
                    #[codec(index = 2)]
                    #[doc = "`:code` was updated."]
                    CodeUpdated,
                    #[codec(index = 3)]
                    #[doc = "A new account was created."]
                    NewAccount {
                        account: ::subxt::ext::sp_core::crypto::AccountId32,
                    },
                    #[codec(index = 4)]
                    #[doc = "An account was reaped."]
                    KilledAccount {
                        account: ::subxt::ext::sp_core::crypto::AccountId32,
                    },
                    #[codec(index = 5)]
                    #[doc = "On on-chain remark happened."]
                    Remarked {
                        sender: ::subxt::ext::sp_core::crypto::AccountId32,
                        hash: ::subxt::ext::sp_core::H256,
                    },
                }
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct AccountInfo<_0, _1> {
                pub nonce: _0,
                pub consumers: _0,
                pub providers: _0,
                pub sufficients: _0,
                pub data: _1,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct EventRecord<_0, _1> {
                pub phase: runtime_types::frame_system::Phase,
                pub event: _0,
                pub topics: ::std::vec::Vec<_1>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct LastRuntimeUpgradeInfo {
                #[codec(compact)]
                pub spec_version: ::core::primitive::u32,
                pub spec_name: ::std::string::String,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum Phase {
                #[codec(index = 0)]
                ApplyExtrinsic(::core::primitive::u32),
                #[codec(index = 1)]
                Finalization,
                #[codec(index = 2)]
                Initialization,
            }
        }
        pub mod pallet_aleph {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
                pub enum Call {
                    #[codec(index = 0)]
                    #[doc = "Sets the emergency finalization key. If called in session `N` the key can be used to"]
                    #[doc = "finalize blocks from session `N+2` onwards, until it gets overridden."]
                    set_emergency_finalizer {
                        emergency_finalizer: runtime_types::primitives::app::Public,
                    },
                    #[codec(index = 1)]
                    #[doc = "Schedules a finality version change for a future session. If such a scheduled future"]
                    #[doc = "version is already set, it is replaced with the provided one."]
                    #[doc = "Any rescheduling of a future version change needs to occur at least 2 sessions in"]
                    #[doc = "advance of the provided session of the version change."]
                    #[doc = "In order to cancel a scheduled version change, a new version change should be scheduled"]
                    #[doc = "with the same version as the current one."]
                    schedule_finality_version_change {
                        version_incoming: ::core::primitive::u32,
                        session: ::core::primitive::u32,
                    },
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
                pub enum Event {
                    #[codec(index = 0)]
                    ChangeEmergencyFinalizer(runtime_types::primitives::app::Public),
                    #[codec(index = 1)]
                    ScheduleFinalityVersionChange(runtime_types::primitives::VersionChange),
                    #[codec(index = 2)]
                    FinalityVersionChange(runtime_types::primitives::VersionChange),
                }
            }
        }
        pub mod pallet_baby_liminal {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
                pub enum Call {
                    #[codec(index = 0)]
                    #[doc = "Stores `key` under `identifier` in `VerificationKeys` map."]
                    #[doc = ""]
                    #[doc = "Fails if:"]
                    #[doc = "- `key.len()` is greater than `MaximumVerificationKeyLength`, or"]
                    #[doc = "- `identifier` has been already used"]
                    #[doc = ""]
                    #[doc = "`key` can come from any proving system - there are no checks that verify it, in"]
                    #[doc = "particular, `key` can contain just trash bytes."]
                    store_key {
                        identifier: [::core::primitive::u8; 4usize],
                        key: ::std::vec::Vec<::core::primitive::u8>,
                    },
                    #[codec(index = 1)]
                    #[doc = "Deletes a key stored under `identifier` in `VerificationKeys` map."]
                    #[doc = ""]
                    #[doc = "Can only be called by a root account."]
                    delete_key {
                        identifier: [::core::primitive::u8; 4usize],
                    },
                    #[codec(index = 2)]
                    #[doc = "Overwrites a key stored under `identifier` in `VerificationKeys` map with a new value `key`"]
                    #[doc = ""]
                    #[doc = "Fails if `key.len()` is greater than `MaximumVerificationKeyLength`."]
                    #[doc = "Can only be called by a root account."]
                    overwrite_key {
                        identifier: [::core::primitive::u8; 4usize],
                        key: ::std::vec::Vec<::core::primitive::u8>,
                    },
                    #[codec(index = 3)]
                    #[doc = "Verifies `proof` against `public_input` with a key that has been stored under"]
                    #[doc = "`verification_key_identifier`. All is done within `system` proving system."]
                    #[doc = ""]
                    #[doc = "Fails if:"]
                    #[doc = "- there is no verification key under `verification_key_identifier`"]
                    #[doc = "- verification key under `verification_key_identifier` cannot be deserialized"]
                    #[doc = "(e.g. it has been produced for another proving system)"]
                    #[doc = "- `proof` cannot be deserialized (e.g. it has been produced for another proving system)"]
                    #[doc = "- `public_input` cannot be deserialized (e.g. it has been produced for another proving"]
                    #[doc = "system)"]
                    #[doc = "- verifying procedure fails (e.g. incompatible verification key and proof)"]
                    #[doc = "- proof is incorrect"]
                    verify {
                        verification_key_identifier: [::core::primitive::u8; 4usize],
                        proof: ::std::vec::Vec<::core::primitive::u8>,
                        public_input: ::std::vec::Vec<::core::primitive::u8>,
                        system: runtime_types::pallet_baby_liminal::systems::ProvingSystem,
                    },
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tCustom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)\n\t\t\tof this pallet.\n\t\t\t"]
                pub enum Error {
                    #[codec(index = 0)]
                    #[doc = "This verification key identifier is already taken."]
                    IdentifierAlreadyInUse,
                    #[codec(index = 1)]
                    #[doc = "There is no verification key available under this identifier."]
                    UnknownVerificationKeyIdentifier,
                    #[codec(index = 2)]
                    #[doc = "Provided verification key is longer than `MaximumVerificationKeyLength` limit."]
                    VerificationKeyTooLong,
                    #[codec(index = 3)]
                    #[doc = "Either proof or public input is longer than `MaximumDataLength` limit."]
                    DataTooLong,
                    #[codec(index = 4)]
                    #[doc = "Couldn't deserialize proof."]
                    DeserializingProofFailed,
                    #[codec(index = 5)]
                    #[doc = "Couldn't deserialize public input."]
                    DeserializingPublicInputFailed,
                    #[codec(index = 6)]
                    #[doc = "Couldn't deserialize verification key from storage."]
                    DeserializingVerificationKeyFailed,
                    #[codec(index = 7)]
                    #[doc = "Verification procedure has failed. Proof still can be correct."]
                    VerificationFailed(
                        runtime_types::pallet_baby_liminal::systems::VerificationError,
                    ),
                    #[codec(index = 8)]
                    #[doc = "Proof has been found as incorrect."]
                    IncorrectProof,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
                pub enum Event {
                    #[codec(index = 0)]
                    #[doc = "Verification key has been successfully stored."]
                    VerificationKeyStored,
                    #[codec(index = 1)]
                    #[doc = "Verification key has been successfully deleted."]
                    VerificationKeyDeleted,
                    #[codec(index = 2)]
                    #[doc = "Verification key has been successfully overwritten."]
                    VerificationKeyOverwritten,
                    #[codec(index = 3)]
                    #[doc = "Proof has been successfully verified."]
                    VerificationSucceeded,
                }
            }
            pub mod systems {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub enum ProvingSystem {
                    #[codec(index = 0)]
                    Groth16,
                    #[codec(index = 1)]
                    Gm17,
                    #[codec(index = 2)]
                    Marlin,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub enum VerificationError {
                    #[codec(index = 0)]
                    MalformedVerifyingKey,
                    #[codec(index = 1)]
                    AHPError,
                    #[codec(index = 2)]
                    PolynomialCommitmentError,
                    #[codec(index = 3)]
                    UnexpectedError,
                }
            }
        }
        pub mod pallet_balances {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
                pub enum Call {
                    #[codec(index = 0)]
                    #[doc = "Transfer some liquid free balance to another account."]
                    #[doc = ""]
                    #[doc = "`transfer` will set the `FreeBalance` of the sender and receiver."]
                    #[doc = "If the sender's account is below the existential deposit as a result"]
                    #[doc = "of the transfer, the account will be reaped."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be `Signed` by the transactor."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- Dependent on arguments but not critical, given proper implementations for input config"]
                    #[doc = "  types. See related functions below."]
                    #[doc = "- It contains a limited number of reads and writes internally and no complex"]
                    #[doc = "  computation."]
                    #[doc = ""]
                    #[doc = "Related functions:"]
                    #[doc = ""]
                    #[doc = "  - `ensure_can_withdraw` is always called internally but has a bounded complexity."]
                    #[doc = "  - Transferring balances to accounts that did not exist before will cause"]
                    #[doc = "    `T::OnNewAccount::on_new_account` to be called."]
                    #[doc = "  - Removing enough funds from an account will trigger `T::DustRemoval::on_unbalanced`."]
                    #[doc = "  - `transfer_keep_alive` works the same way as `transfer`, but has an additional check"]
                    #[doc = "    that the transfer will not kill the origin account."]
                    #[doc = "---------------------------------"]
                    #[doc = "- Origin account is already in memory, so no DB operations for them."]
                    #[doc = "# </weight>"]
                    transfer {
                        dest: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        #[codec(compact)]
                        value: ::core::primitive::u128,
                    },
                    #[codec(index = 1)]
                    #[doc = "Set the balances of a given account."]
                    #[doc = ""]
                    #[doc = "This will alter `FreeBalance` and `ReservedBalance` in storage. it will"]
                    #[doc = "also alter the total issuance of the system (`TotalIssuance`) appropriately."]
                    #[doc = "If the new free or reserved balance is below the existential deposit,"]
                    #[doc = "it will reset the account nonce (`frame_system::AccountNonce`)."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call is `root`."]
                    set_balance {
                        who: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        #[codec(compact)]
                        new_free: ::core::primitive::u128,
                        #[codec(compact)]
                        new_reserved: ::core::primitive::u128,
                    },
                    #[codec(index = 2)]
                    #[doc = "Exactly as `transfer`, except the origin must be root and the source account may be"]
                    #[doc = "specified."]
                    #[doc = "# <weight>"]
                    #[doc = "- Same as transfer, but additional read and write because the source account is not"]
                    #[doc = "  assumed to be in the overlay."]
                    #[doc = "# </weight>"]
                    force_transfer {
                        source: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        dest: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        #[codec(compact)]
                        value: ::core::primitive::u128,
                    },
                    #[codec(index = 3)]
                    #[doc = "Same as the [`transfer`] call, but with a check that the transfer will not kill the"]
                    #[doc = "origin account."]
                    #[doc = ""]
                    #[doc = "99% of the time you want [`transfer`] instead."]
                    #[doc = ""]
                    #[doc = "[`transfer`]: struct.Pallet.html#method.transfer"]
                    transfer_keep_alive {
                        dest: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        #[codec(compact)]
                        value: ::core::primitive::u128,
                    },
                    #[codec(index = 4)]
                    #[doc = "Transfer the entire transferable balance from the caller account."]
                    #[doc = ""]
                    #[doc = "NOTE: This function only attempts to transfer _transferable_ balances. This means that"]
                    #[doc = "any locked, reserved, or existential deposits (when `keep_alive` is `true`), will not be"]
                    #[doc = "transferred by this function. To ensure that this function results in a killed account,"]
                    #[doc = "you might need to prepare the account by removing any reference counters, storage"]
                    #[doc = "deposits, etc..."]
                    #[doc = ""]
                    #[doc = "The dispatch origin of this call must be Signed."]
                    #[doc = ""]
                    #[doc = "- `dest`: The recipient of the transfer."]
                    #[doc = "- `keep_alive`: A boolean to determine if the `transfer_all` operation should send all"]
                    #[doc = "  of the funds the account has, causing the sender account to be killed (false), or"]
                    #[doc = "  transfer everything except at least the existential deposit, which will guarantee to"]
                    #[doc = "  keep the sender account alive (true). # <weight>"]
                    #[doc = "- O(1). Just like transfer, but reading the user's transferable balance first."]
                    #[doc = "  #</weight>"]
                    transfer_all {
                        dest: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        keep_alive: ::core::primitive::bool,
                    },
                    #[codec(index = 5)]
                    #[doc = "Unreserve some balance from a user by force."]
                    #[doc = ""]
                    #[doc = "Can only be called by ROOT."]
                    force_unreserve {
                        who: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        amount: ::core::primitive::u128,
                    },
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tCustom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)\n\t\t\tof this pallet.\n\t\t\t"]
                pub enum Error {
                    #[codec(index = 0)]
                    #[doc = "Vesting balance too high to send value"]
                    VestingBalance,
                    #[codec(index = 1)]
                    #[doc = "Account liquidity restrictions prevent withdrawal"]
                    LiquidityRestrictions,
                    #[codec(index = 2)]
                    #[doc = "Balance too low to send value."]
                    InsufficientBalance,
                    #[codec(index = 3)]
                    #[doc = "Value too low to create account due to existential deposit"]
                    ExistentialDeposit,
                    #[codec(index = 4)]
                    #[doc = "Transfer/payment would kill account"]
                    KeepAlive,
                    #[codec(index = 5)]
                    #[doc = "A vesting schedule already exists for this account"]
                    ExistingVestingSchedule,
                    #[codec(index = 6)]
                    #[doc = "Beneficiary account must pre-exist"]
                    DeadAccount,
                    #[codec(index = 7)]
                    #[doc = "Number of named reserves exceed MaxReserves"]
                    TooManyReserves,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
                pub enum Event {
                    #[codec(index = 0)]
                    #[doc = "An account was created with some free balance."]
                    Endowed {
                        account: ::subxt::ext::sp_core::crypto::AccountId32,
                        free_balance: ::core::primitive::u128,
                    },
                    #[codec(index = 1)]
                    #[doc = "An account was removed whose balance was non-zero but below ExistentialDeposit,"]
                    #[doc = "resulting in an outright loss."]
                    DustLost {
                        account: ::subxt::ext::sp_core::crypto::AccountId32,
                        amount: ::core::primitive::u128,
                    },
                    #[codec(index = 2)]
                    #[doc = "Transfer succeeded."]
                    Transfer {
                        from: ::subxt::ext::sp_core::crypto::AccountId32,
                        to: ::subxt::ext::sp_core::crypto::AccountId32,
                        amount: ::core::primitive::u128,
                    },
                    #[codec(index = 3)]
                    #[doc = "A balance was set by root."]
                    BalanceSet {
                        who: ::subxt::ext::sp_core::crypto::AccountId32,
                        free: ::core::primitive::u128,
                        reserved: ::core::primitive::u128,
                    },
                    #[codec(index = 4)]
                    #[doc = "Some balance was reserved (moved from free to reserved)."]
                    Reserved {
                        who: ::subxt::ext::sp_core::crypto::AccountId32,
                        amount: ::core::primitive::u128,
                    },
                    #[codec(index = 5)]
                    #[doc = "Some balance was unreserved (moved from reserved to free)."]
                    Unreserved {
                        who: ::subxt::ext::sp_core::crypto::AccountId32,
                        amount: ::core::primitive::u128,
                    },
                    #[codec(index = 6)]
                    #[doc = "Some balance was moved from the reserve of the first account to the second account."]
                    #[doc = "Final argument indicates the destination balance type."]
                    ReserveRepatriated {
                        from: ::subxt::ext::sp_core::crypto::AccountId32,
                        to: ::subxt::ext::sp_core::crypto::AccountId32,
                        amount: ::core::primitive::u128,
                        destination_status:
                            runtime_types::frame_support::traits::tokens::misc::BalanceStatus,
                    },
                    #[codec(index = 7)]
                    #[doc = "Some amount was deposited (e.g. for transaction fees)."]
                    Deposit {
                        who: ::subxt::ext::sp_core::crypto::AccountId32,
                        amount: ::core::primitive::u128,
                    },
                    #[codec(index = 8)]
                    #[doc = "Some amount was withdrawn from the account (e.g. for transaction fees)."]
                    Withdraw {
                        who: ::subxt::ext::sp_core::crypto::AccountId32,
                        amount: ::core::primitive::u128,
                    },
                    #[codec(index = 9)]
                    #[doc = "Some amount was removed from the account (e.g. for misbehavior)."]
                    Slashed {
                        who: ::subxt::ext::sp_core::crypto::AccountId32,
                        amount: ::core::primitive::u128,
                    },
                }
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct AccountData<_0> {
                pub free: _0,
                pub reserved: _0,
                pub misc_frozen: _0,
                pub fee_frozen: _0,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct BalanceLock<_0> {
                pub id: [::core::primitive::u8; 8usize],
                pub amount: _0,
                pub reasons: runtime_types::pallet_balances::Reasons,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum Reasons {
                #[codec(index = 0)]
                Fee,
                #[codec(index = 1)]
                Misc,
                #[codec(index = 2)]
                All,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ReserveData<_0, _1> {
                pub id: _0,
                pub amount: _1,
            }
        }
        pub mod pallet_contracts {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
                pub enum Call {
                    #[codec(index = 0)]
                    #[doc = "Deprecated version if [`Self::call`] for use in an in-storage `Call`."]
                    call_old_weight {
                        dest: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        #[codec(compact)]
                        value: ::core::primitive::u128,
                        #[codec(compact)]
                        gas_limit: runtime_types::sp_weights::OldWeight,
                        storage_deposit_limit: ::core::option::Option<
                            ::subxt::ext::codec::Compact<::core::primitive::u128>,
                        >,
                        data: ::std::vec::Vec<::core::primitive::u8>,
                    },
                    #[codec(index = 1)]
                    #[doc = "Deprecated version if [`Self::instantiate_with_code`] for use in an in-storage `Call`."]
                    instantiate_with_code_old_weight {
                        #[codec(compact)]
                        value: ::core::primitive::u128,
                        #[codec(compact)]
                        gas_limit: runtime_types::sp_weights::OldWeight,
                        storage_deposit_limit: ::core::option::Option<
                            ::subxt::ext::codec::Compact<::core::primitive::u128>,
                        >,
                        code: ::std::vec::Vec<::core::primitive::u8>,
                        data: ::std::vec::Vec<::core::primitive::u8>,
                        salt: ::std::vec::Vec<::core::primitive::u8>,
                    },
                    #[codec(index = 2)]
                    #[doc = "Deprecated version if [`Self::instantiate`] for use in an in-storage `Call`."]
                    instantiate_old_weight {
                        #[codec(compact)]
                        value: ::core::primitive::u128,
                        #[codec(compact)]
                        gas_limit: runtime_types::sp_weights::OldWeight,
                        storage_deposit_limit: ::core::option::Option<
                            ::subxt::ext::codec::Compact<::core::primitive::u128>,
                        >,
                        code_hash: ::subxt::ext::sp_core::H256,
                        data: ::std::vec::Vec<::core::primitive::u8>,
                        salt: ::std::vec::Vec<::core::primitive::u8>,
                    },
                    #[codec(index = 3)]
                    #[doc = "Upload new `code` without instantiating a contract from it."]
                    #[doc = ""]
                    #[doc = "If the code does not already exist a deposit is reserved from the caller"]
                    #[doc = "and unreserved only when [`Self::remove_code`] is called. The size of the reserve"]
                    #[doc = "depends on the instrumented size of the the supplied `code`."]
                    #[doc = ""]
                    #[doc = "If the code already exists in storage it will still return `Ok` and upgrades"]
                    #[doc = "the in storage version to the current"]
                    #[doc = "[`InstructionWeights::version`](InstructionWeights)."]
                    #[doc = ""]
                    #[doc = "- `determinism`: If this is set to any other value but [`Determinism::Deterministic`]"]
                    #[doc = "  then the only way to use this code is to delegate call into it from an offchain"]
                    #[doc = "  execution. Set to [`Determinism::Deterministic`] if in doubt."]
                    #[doc = ""]
                    #[doc = "# Note"]
                    #[doc = ""]
                    #[doc = "Anyone can instantiate a contract from any uploaded code and thus prevent its removal."]
                    #[doc = "To avoid this situation a constructor could employ access control so that it can"]
                    #[doc = "only be instantiated by permissioned entities. The same is true when uploading"]
                    #[doc = "through [`Self::instantiate_with_code`]."]
                    upload_code {
                        code: ::std::vec::Vec<::core::primitive::u8>,
                        storage_deposit_limit: ::core::option::Option<
                            ::subxt::ext::codec::Compact<::core::primitive::u128>,
                        >,
                        determinism: runtime_types::pallet_contracts::wasm::Determinism,
                    },
                    #[codec(index = 4)]
                    #[doc = "Remove the code stored under `code_hash` and refund the deposit to its owner."]
                    #[doc = ""]
                    #[doc = "A code can only be removed by its original uploader (its owner) and only if it is"]
                    #[doc = "not used by any contract."]
                    remove_code {
                        code_hash: ::subxt::ext::sp_core::H256,
                    },
                    #[codec(index = 5)]
                    #[doc = "Privileged function that changes the code of an existing contract."]
                    #[doc = ""]
                    #[doc = "This takes care of updating refcounts and all other necessary operations. Returns"]
                    #[doc = "an error if either the `code_hash` or `dest` do not exist."]
                    #[doc = ""]
                    #[doc = "# Note"]
                    #[doc = ""]
                    #[doc = "This does **not** change the address of the contract in question. This means"]
                    #[doc = "that the contract address is no longer derived from its code hash after calling"]
                    #[doc = "this dispatchable."]
                    set_code {
                        dest: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        code_hash: ::subxt::ext::sp_core::H256,
                    },
                    #[codec(index = 6)]
                    #[doc = "Makes a call to an account, optionally transferring some balance."]
                    #[doc = ""]
                    #[doc = "# Parameters"]
                    #[doc = ""]
                    #[doc = "* `dest`: Address of the contract to call."]
                    #[doc = "* `value`: The balance to transfer from the `origin` to `dest`."]
                    #[doc = "* `gas_limit`: The gas limit enforced when executing the constructor."]
                    #[doc = "* `storage_deposit_limit`: The maximum amount of balance that can be charged from the"]
                    #[doc = "  caller to pay for the storage consumed."]
                    #[doc = "* `data`: The input data to pass to the contract."]
                    #[doc = ""]
                    #[doc = "* If the account is a smart-contract account, the associated code will be"]
                    #[doc = "executed and any value will be transferred."]
                    #[doc = "* If the account is a regular account, any value will be transferred."]
                    #[doc = "* If no account exists and the call value is not less than `existential_deposit`,"]
                    #[doc = "a regular account will be created and any value will be transferred."]
                    call {
                        dest: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        #[codec(compact)]
                        value: ::core::primitive::u128,
                        gas_limit: runtime_types::sp_weights::weight_v2::Weight,
                        storage_deposit_limit: ::core::option::Option<
                            ::subxt::ext::codec::Compact<::core::primitive::u128>,
                        >,
                        data: ::std::vec::Vec<::core::primitive::u8>,
                    },
                    #[codec(index = 7)]
                    #[doc = "Instantiates a new contract from the supplied `code` optionally transferring"]
                    #[doc = "some balance."]
                    #[doc = ""]
                    #[doc = "This dispatchable has the same effect as calling [`Self::upload_code`] +"]
                    #[doc = "[`Self::instantiate`]. Bundling them together provides efficiency gains. Please"]
                    #[doc = "also check the documentation of [`Self::upload_code`]."]
                    #[doc = ""]
                    #[doc = "# Parameters"]
                    #[doc = ""]
                    #[doc = "* `value`: The balance to transfer from the `origin` to the newly created contract."]
                    #[doc = "* `gas_limit`: The gas limit enforced when executing the constructor."]
                    #[doc = "* `storage_deposit_limit`: The maximum amount of balance that can be charged/reserved"]
                    #[doc = "  from the caller to pay for the storage consumed."]
                    #[doc = "* `code`: The contract code to deploy in raw bytes."]
                    #[doc = "* `data`: The input data to pass to the contract constructor."]
                    #[doc = "* `salt`: Used for the address derivation. See [`Pallet::contract_address`]."]
                    #[doc = ""]
                    #[doc = "Instantiation is executed as follows:"]
                    #[doc = ""]
                    #[doc = "- The supplied `code` is instrumented, deployed, and a `code_hash` is created for that"]
                    #[doc = "  code."]
                    #[doc = "- If the `code_hash` already exists on the chain the underlying `code` will be shared."]
                    #[doc = "- The destination address is computed based on the sender, code_hash and the salt."]
                    #[doc = "- The smart-contract account is created at the computed address."]
                    #[doc = "- The `value` is transferred to the new account."]
                    #[doc = "- The `deploy` function is executed in the context of the newly-created account."]
                    instantiate_with_code {
                        #[codec(compact)]
                        value: ::core::primitive::u128,
                        gas_limit: runtime_types::sp_weights::weight_v2::Weight,
                        storage_deposit_limit: ::core::option::Option<
                            ::subxt::ext::codec::Compact<::core::primitive::u128>,
                        >,
                        code: ::std::vec::Vec<::core::primitive::u8>,
                        data: ::std::vec::Vec<::core::primitive::u8>,
                        salt: ::std::vec::Vec<::core::primitive::u8>,
                    },
                    #[codec(index = 8)]
                    #[doc = "Instantiates a contract from a previously deployed wasm binary."]
                    #[doc = ""]
                    #[doc = "This function is identical to [`Self::instantiate_with_code`] but without the"]
                    #[doc = "code deployment step. Instead, the `code_hash` of an on-chain deployed wasm binary"]
                    #[doc = "must be supplied."]
                    instantiate {
                        #[codec(compact)]
                        value: ::core::primitive::u128,
                        gas_limit: runtime_types::sp_weights::weight_v2::Weight,
                        storage_deposit_limit: ::core::option::Option<
                            ::subxt::ext::codec::Compact<::core::primitive::u128>,
                        >,
                        code_hash: ::subxt::ext::sp_core::H256,
                        data: ::std::vec::Vec<::core::primitive::u8>,
                        salt: ::std::vec::Vec<::core::primitive::u8>,
                    },
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tCustom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)\n\t\t\tof this pallet.\n\t\t\t"]
                pub enum Error {
                    #[codec(index = 0)]
                    #[doc = "A new schedule must have a greater version than the current one."]
                    InvalidScheduleVersion,
                    #[codec(index = 1)]
                    #[doc = "Invalid combination of flags supplied to `seal_call` or `seal_delegate_call`."]
                    InvalidCallFlags,
                    #[codec(index = 2)]
                    #[doc = "The executed contract exhausted its gas limit."]
                    OutOfGas,
                    #[codec(index = 3)]
                    #[doc = "The output buffer supplied to a contract API call was too small."]
                    OutputBufferTooSmall,
                    #[codec(index = 4)]
                    #[doc = "Performing the requested transfer failed. Probably because there isn't enough"]
                    #[doc = "free balance in the sender's account."]
                    TransferFailed,
                    #[codec(index = 5)]
                    #[doc = "Performing a call was denied because the calling depth reached the limit"]
                    #[doc = "of what is specified in the schedule."]
                    MaxCallDepthReached,
                    #[codec(index = 6)]
                    #[doc = "No contract was found at the specified address."]
                    ContractNotFound,
                    #[codec(index = 7)]
                    #[doc = "The code supplied to `instantiate_with_code` exceeds the limit specified in the"]
                    #[doc = "current schedule."]
                    CodeTooLarge,
                    #[codec(index = 8)]
                    #[doc = "No code could be found at the supplied code hash."]
                    CodeNotFound,
                    #[codec(index = 9)]
                    #[doc = "A buffer outside of sandbox memory was passed to a contract API function."]
                    OutOfBounds,
                    #[codec(index = 10)]
                    #[doc = "Input passed to a contract API function failed to decode as expected type."]
                    DecodingFailed,
                    #[codec(index = 11)]
                    #[doc = "Contract trapped during execution."]
                    ContractTrapped,
                    #[codec(index = 12)]
                    #[doc = "The size defined in `T::MaxValueSize` was exceeded."]
                    ValueTooLarge,
                    #[codec(index = 13)]
                    #[doc = "Termination of a contract is not allowed while the contract is already"]
                    #[doc = "on the call stack. Can be triggered by `seal_terminate`."]
                    TerminatedWhileReentrant,
                    #[codec(index = 14)]
                    #[doc = "`seal_call` forwarded this contracts input. It therefore is no longer available."]
                    InputForwarded,
                    #[codec(index = 15)]
                    #[doc = "The subject passed to `seal_random` exceeds the limit."]
                    RandomSubjectTooLong,
                    #[codec(index = 16)]
                    #[doc = "The amount of topics passed to `seal_deposit_events` exceeds the limit."]
                    TooManyTopics,
                    #[codec(index = 17)]
                    #[doc = "The chain does not provide a chain extension. Calling the chain extension results"]
                    #[doc = "in this error. Note that this usually  shouldn't happen as deploying such contracts"]
                    #[doc = "is rejected."]
                    NoChainExtension,
                    #[codec(index = 18)]
                    #[doc = "Removal of a contract failed because the deletion queue is full."]
                    #[doc = ""]
                    #[doc = "This can happen when calling `seal_terminate`."]
                    #[doc = "The queue is filled by deleting contracts and emptied by a fixed amount each block."]
                    #[doc = "Trying again during another block is the only way to resolve this issue."]
                    DeletionQueueFull,
                    #[codec(index = 19)]
                    #[doc = "A contract with the same AccountId already exists."]
                    DuplicateContract,
                    #[codec(index = 20)]
                    #[doc = "A contract self destructed in its constructor."]
                    #[doc = ""]
                    #[doc = "This can be triggered by a call to `seal_terminate`."]
                    TerminatedInConstructor,
                    #[codec(index = 21)]
                    #[doc = "The debug message specified to `seal_debug_message` does contain invalid UTF-8."]
                    DebugMessageInvalidUTF8,
                    #[codec(index = 22)]
                    #[doc = "A call tried to invoke a contract that is flagged as non-reentrant."]
                    ReentranceDenied,
                    #[codec(index = 23)]
                    #[doc = "Origin doesn't have enough balance to pay the required storage deposits."]
                    StorageDepositNotEnoughFunds,
                    #[codec(index = 24)]
                    #[doc = "More storage was created than allowed by the storage deposit limit."]
                    StorageDepositLimitExhausted,
                    #[codec(index = 25)]
                    #[doc = "Code removal was denied because the code is still in use by at least one contract."]
                    CodeInUse,
                    #[codec(index = 26)]
                    #[doc = "The contract ran to completion but decided to revert its storage changes."]
                    #[doc = "Please note that this error is only returned from extrinsics. When called directly"]
                    #[doc = "or via RPC an `Ok` will be returned. In this case the caller needs to inspect the flags"]
                    #[doc = "to determine whether a reversion has taken place."]
                    ContractReverted,
                    #[codec(index = 27)]
                    #[doc = "The contract's code was found to be invalid during validation or instrumentation."]
                    #[doc = ""]
                    #[doc = "The most likely cause of this is that an API was used which is not supported by the"]
                    #[doc = "node. This hapens if an older node is used with a new version of ink!. Try updating"]
                    #[doc = "your node to the newest available version."]
                    #[doc = ""]
                    #[doc = "A more detailed error can be found on the node console if debug messages are enabled"]
                    #[doc = "by supplying `-lruntime::contracts=debug`."]
                    CodeRejected,
                    #[codec(index = 28)]
                    #[doc = "An indetermistic code was used in a context where this is not permitted."]
                    Indeterministic,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
                pub enum Event {
                    #[codec(index = 0)]
                    #[doc = "Contract deployed by address at the specified address."]
                    Instantiated {
                        deployer: ::subxt::ext::sp_core::crypto::AccountId32,
                        contract: ::subxt::ext::sp_core::crypto::AccountId32,
                    },
                    #[codec(index = 1)]
                    #[doc = "Contract has been removed."]
                    #[doc = ""]
                    #[doc = "# Note"]
                    #[doc = ""]
                    #[doc = "The only way for a contract to be removed and emitting this event is by calling"]
                    #[doc = "`seal_terminate`."]
                    Terminated {
                        contract: ::subxt::ext::sp_core::crypto::AccountId32,
                        beneficiary: ::subxt::ext::sp_core::crypto::AccountId32,
                    },
                    #[codec(index = 2)]
                    #[doc = "Code with the specified hash has been stored."]
                    CodeStored {
                        code_hash: ::subxt::ext::sp_core::H256,
                    },
                    #[codec(index = 3)]
                    #[doc = "A custom event emitted by the contract."]
                    ContractEmitted {
                        contract: ::subxt::ext::sp_core::crypto::AccountId32,
                        data: ::std::vec::Vec<::core::primitive::u8>,
                    },
                    #[codec(index = 4)]
                    #[doc = "A code with the specified hash was removed."]
                    CodeRemoved {
                        code_hash: ::subxt::ext::sp_core::H256,
                    },
                    #[codec(index = 5)]
                    #[doc = "A contract's code was updated."]
                    ContractCodeUpdated {
                        contract: ::subxt::ext::sp_core::crypto::AccountId32,
                        new_code_hash: ::subxt::ext::sp_core::H256,
                        old_code_hash: ::subxt::ext::sp_core::H256,
                    },
                    #[codec(index = 6)]
                    #[doc = "A contract was called either by a plain account or another contract."]
                    #[doc = ""]
                    #[doc = "# Note"]
                    #[doc = ""]
                    #[doc = "Please keep in mind that like all events this is only emitted for successful"]
                    #[doc = "calls. This is because on failure all storage changes including events are"]
                    #[doc = "rolled back."]
                    Called {
                        caller: ::subxt::ext::sp_core::crypto::AccountId32,
                        contract: ::subxt::ext::sp_core::crypto::AccountId32,
                    },
                    #[codec(index = 7)]
                    #[doc = "A contract delegate called a code hash."]
                    #[doc = ""]
                    #[doc = "# Note"]
                    #[doc = ""]
                    #[doc = "Please keep in mind that like all events this is only emitted for successful"]
                    #[doc = "calls. This is because on failure all storage changes including events are"]
                    #[doc = "rolled back."]
                    DelegateCalled {
                        contract: ::subxt::ext::sp_core::crypto::AccountId32,
                        code_hash: ::subxt::ext::sp_core::H256,
                    },
                }
            }
            pub mod schedule {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct HostFnWeights {
                    pub caller: runtime_types::sp_weights::weight_v2::Weight,
                    pub is_contract: runtime_types::sp_weights::weight_v2::Weight,
                    pub code_hash: runtime_types::sp_weights::weight_v2::Weight,
                    pub own_code_hash: runtime_types::sp_weights::weight_v2::Weight,
                    pub caller_is_origin: runtime_types::sp_weights::weight_v2::Weight,
                    pub address: runtime_types::sp_weights::weight_v2::Weight,
                    pub gas_left: runtime_types::sp_weights::weight_v2::Weight,
                    pub balance: runtime_types::sp_weights::weight_v2::Weight,
                    pub value_transferred: runtime_types::sp_weights::weight_v2::Weight,
                    pub minimum_balance: runtime_types::sp_weights::weight_v2::Weight,
                    pub block_number: runtime_types::sp_weights::weight_v2::Weight,
                    pub now: runtime_types::sp_weights::weight_v2::Weight,
                    pub weight_to_fee: runtime_types::sp_weights::weight_v2::Weight,
                    pub gas: runtime_types::sp_weights::weight_v2::Weight,
                    pub input: runtime_types::sp_weights::weight_v2::Weight,
                    pub input_per_byte: runtime_types::sp_weights::weight_v2::Weight,
                    pub r#return: runtime_types::sp_weights::weight_v2::Weight,
                    pub return_per_byte: runtime_types::sp_weights::weight_v2::Weight,
                    pub terminate: runtime_types::sp_weights::weight_v2::Weight,
                    pub random: runtime_types::sp_weights::weight_v2::Weight,
                    pub deposit_event: runtime_types::sp_weights::weight_v2::Weight,
                    pub deposit_event_per_topic: runtime_types::sp_weights::weight_v2::Weight,
                    pub deposit_event_per_byte: runtime_types::sp_weights::weight_v2::Weight,
                    pub debug_message: runtime_types::sp_weights::weight_v2::Weight,
                    pub set_storage: runtime_types::sp_weights::weight_v2::Weight,
                    pub set_storage_per_new_byte: runtime_types::sp_weights::weight_v2::Weight,
                    pub set_storage_per_old_byte: runtime_types::sp_weights::weight_v2::Weight,
                    pub set_code_hash: runtime_types::sp_weights::weight_v2::Weight,
                    pub clear_storage: runtime_types::sp_weights::weight_v2::Weight,
                    pub clear_storage_per_byte: runtime_types::sp_weights::weight_v2::Weight,
                    pub contains_storage: runtime_types::sp_weights::weight_v2::Weight,
                    pub contains_storage_per_byte: runtime_types::sp_weights::weight_v2::Weight,
                    pub get_storage: runtime_types::sp_weights::weight_v2::Weight,
                    pub get_storage_per_byte: runtime_types::sp_weights::weight_v2::Weight,
                    pub take_storage: runtime_types::sp_weights::weight_v2::Weight,
                    pub take_storage_per_byte: runtime_types::sp_weights::weight_v2::Weight,
                    pub transfer: runtime_types::sp_weights::weight_v2::Weight,
                    pub call: runtime_types::sp_weights::weight_v2::Weight,
                    pub delegate_call: runtime_types::sp_weights::weight_v2::Weight,
                    pub call_transfer_surcharge: runtime_types::sp_weights::weight_v2::Weight,
                    pub call_per_cloned_byte: runtime_types::sp_weights::weight_v2::Weight,
                    pub instantiate: runtime_types::sp_weights::weight_v2::Weight,
                    pub instantiate_transfer_surcharge:
                        runtime_types::sp_weights::weight_v2::Weight,
                    pub instantiate_per_input_byte: runtime_types::sp_weights::weight_v2::Weight,
                    pub instantiate_per_salt_byte: runtime_types::sp_weights::weight_v2::Weight,
                    pub hash_sha2_256: runtime_types::sp_weights::weight_v2::Weight,
                    pub hash_sha2_256_per_byte: runtime_types::sp_weights::weight_v2::Weight,
                    pub hash_keccak_256: runtime_types::sp_weights::weight_v2::Weight,
                    pub hash_keccak_256_per_byte: runtime_types::sp_weights::weight_v2::Weight,
                    pub hash_blake2_256: runtime_types::sp_weights::weight_v2::Weight,
                    pub hash_blake2_256_per_byte: runtime_types::sp_weights::weight_v2::Weight,
                    pub hash_blake2_128: runtime_types::sp_weights::weight_v2::Weight,
                    pub hash_blake2_128_per_byte: runtime_types::sp_weights::weight_v2::Weight,
                    pub ecdsa_recover: runtime_types::sp_weights::weight_v2::Weight,
                    pub ecdsa_to_eth_address: runtime_types::sp_weights::weight_v2::Weight,
                    pub reentrance_count: runtime_types::sp_weights::weight_v2::Weight,
                    pub account_reentrance_count: runtime_types::sp_weights::weight_v2::Weight,
                    pub instantiation_nonce: runtime_types::sp_weights::weight_v2::Weight,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct InstructionWeights {
                    pub version: ::core::primitive::u32,
                    pub fallback: ::core::primitive::u32,
                    pub i64const: ::core::primitive::u32,
                    pub i64load: ::core::primitive::u32,
                    pub i64store: ::core::primitive::u32,
                    pub select: ::core::primitive::u32,
                    pub r#if: ::core::primitive::u32,
                    pub br: ::core::primitive::u32,
                    pub br_if: ::core::primitive::u32,
                    pub br_table: ::core::primitive::u32,
                    pub br_table_per_entry: ::core::primitive::u32,
                    pub call: ::core::primitive::u32,
                    pub call_indirect: ::core::primitive::u32,
                    pub call_indirect_per_param: ::core::primitive::u32,
                    pub call_per_local: ::core::primitive::u32,
                    pub local_get: ::core::primitive::u32,
                    pub local_set: ::core::primitive::u32,
                    pub local_tee: ::core::primitive::u32,
                    pub global_get: ::core::primitive::u32,
                    pub global_set: ::core::primitive::u32,
                    pub memory_current: ::core::primitive::u32,
                    pub memory_grow: ::core::primitive::u32,
                    pub i64clz: ::core::primitive::u32,
                    pub i64ctz: ::core::primitive::u32,
                    pub i64popcnt: ::core::primitive::u32,
                    pub i64eqz: ::core::primitive::u32,
                    pub i64extendsi32: ::core::primitive::u32,
                    pub i64extendui32: ::core::primitive::u32,
                    pub i32wrapi64: ::core::primitive::u32,
                    pub i64eq: ::core::primitive::u32,
                    pub i64ne: ::core::primitive::u32,
                    pub i64lts: ::core::primitive::u32,
                    pub i64ltu: ::core::primitive::u32,
                    pub i64gts: ::core::primitive::u32,
                    pub i64gtu: ::core::primitive::u32,
                    pub i64les: ::core::primitive::u32,
                    pub i64leu: ::core::primitive::u32,
                    pub i64ges: ::core::primitive::u32,
                    pub i64geu: ::core::primitive::u32,
                    pub i64add: ::core::primitive::u32,
                    pub i64sub: ::core::primitive::u32,
                    pub i64mul: ::core::primitive::u32,
                    pub i64divs: ::core::primitive::u32,
                    pub i64divu: ::core::primitive::u32,
                    pub i64rems: ::core::primitive::u32,
                    pub i64remu: ::core::primitive::u32,
                    pub i64and: ::core::primitive::u32,
                    pub i64or: ::core::primitive::u32,
                    pub i64xor: ::core::primitive::u32,
                    pub i64shl: ::core::primitive::u32,
                    pub i64shrs: ::core::primitive::u32,
                    pub i64shru: ::core::primitive::u32,
                    pub i64rotl: ::core::primitive::u32,
                    pub i64rotr: ::core::primitive::u32,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct Limits {
                    pub event_topics: ::core::primitive::u32,
                    pub globals: ::core::primitive::u32,
                    pub locals: ::core::primitive::u32,
                    pub parameters: ::core::primitive::u32,
                    pub memory_pages: ::core::primitive::u32,
                    pub table_size: ::core::primitive::u32,
                    pub br_table_size: ::core::primitive::u32,
                    pub subject_len: ::core::primitive::u32,
                    pub payload_len: ::core::primitive::u32,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct Schedule {
                    pub limits: runtime_types::pallet_contracts::schedule::Limits,
                    pub instruction_weights:
                        runtime_types::pallet_contracts::schedule::InstructionWeights,
                    pub host_fn_weights: runtime_types::pallet_contracts::schedule::HostFnWeights,
                }
            }
            pub mod storage {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct ContractInfo {
                    pub trie_id: runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                        ::core::primitive::u8,
                    >,
                    pub code_hash: ::subxt::ext::sp_core::H256,
                    pub storage_bytes: ::core::primitive::u32,
                    pub storage_items: ::core::primitive::u32,
                    pub storage_byte_deposit: ::core::primitive::u128,
                    pub storage_item_deposit: ::core::primitive::u128,
                    pub storage_base_deposit: ::core::primitive::u128,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct DeletedContract {
                    pub trie_id: runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                        ::core::primitive::u8,
                    >,
                }
            }
            pub mod wasm {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub enum Determinism {
                    #[codec(index = 0)]
                    Deterministic,
                    #[codec(index = 1)]
                    AllowIndeterminism,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct OwnerInfo {
                    pub owner: ::subxt::ext::sp_core::crypto::AccountId32,
                    #[codec(compact)]
                    pub deposit: ::core::primitive::u128,
                    #[codec(compact)]
                    pub refcount: ::core::primitive::u64,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct PrefabWasmModule {
                    #[codec(compact)]
                    pub instruction_weights_version: ::core::primitive::u32,
                    #[codec(compact)]
                    pub initial: ::core::primitive::u32,
                    #[codec(compact)]
                    pub maximum: ::core::primitive::u32,
                    pub code: runtime_types::sp_core::bounded::weak_bounded_vec::WeakBoundedVec<
                        ::core::primitive::u8,
                    >,
                    pub determinism: runtime_types::pallet_contracts::wasm::Determinism,
                }
            }
        }
        pub mod pallet_elections {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
                pub enum Call {
                    #[codec(index = 0)]
                    change_validators {
                        reserved_validators: ::core::option::Option<
                            ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                        >,
                        non_reserved_validators: ::core::option::Option<
                            ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                        >,
                        committee_size:
                            ::core::option::Option<runtime_types::primitives::CommitteeSeats>,
                    },
                    #[codec(index = 1)]
                    #[doc = "Sets ban config, it has an immediate effect"]
                    set_ban_config {
                        minimal_expected_performance: ::core::option::Option<::core::primitive::u8>,
                        underperformed_session_count_threshold:
                            ::core::option::Option<::core::primitive::u32>,
                        clean_session_counter_delay: ::core::option::Option<::core::primitive::u32>,
                        ban_period: ::core::option::Option<::core::primitive::u32>,
                    },
                    #[codec(index = 2)]
                    #[doc = "Schedule a non-reserved node to be banned out from the committee at the end of the era"]
                    ban_from_committee {
                        banned: ::subxt::ext::sp_core::crypto::AccountId32,
                        ban_reason: ::std::vec::Vec<::core::primitive::u8>,
                    },
                    #[codec(index = 3)]
                    #[doc = "Schedule a non-reserved node to be banned out from the committee at the end of the era"]
                    cancel_ban {
                        banned: ::subxt::ext::sp_core::crypto::AccountId32,
                    },
                    #[codec(index = 4)]
                    #[doc = "Set openness of the elections"]
                    set_elections_openness {
                        openness: runtime_types::primitives::ElectionOpenness,
                    },
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tCustom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)\n\t\t\tof this pallet.\n\t\t\t"]
                pub enum Error {
                    #[codec(index = 0)]
                    NotEnoughValidators,
                    #[codec(index = 1)]
                    NotEnoughReservedValidators,
                    #[codec(index = 2)]
                    NotEnoughNonReservedValidators,
                    #[codec(index = 3)]
                    NonUniqueListOfValidators,
                    #[codec(index = 4)]
                    #[doc = "Raised in any scenario [`BanConfig`] is invalid"]
                    #[doc = "* `performance_ratio_threshold` must be a number in range [0; 100]"]
                    #[doc = "* `underperformed_session_count_threshold` must be a positive number,"]
                    #[doc = "* `clean_session_counter_delay` must be a positive number."]
                    InvalidBanConfig,
                    #[codec(index = 5)]
                    #[doc = "Ban reason is too big, ie given vector of bytes is greater than"]
                    #[doc = "[`Config::MaximumBanReasonLength`]"]
                    BanReasonTooBig,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
                pub enum Event {
                    #[codec(index = 0)]
                    #[doc = "Committee for the next era has changed"]
                    ChangeValidators(
                        ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                        ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                        runtime_types::primitives::CommitteeSeats,
                    ),
                    #[codec(index = 1)]
                    #[doc = "Ban thresholds for the next era has changed"]
                    SetBanConfig(runtime_types::primitives::BanConfig),
                    #[codec(index = 2)]
                    #[doc = "Validators have been banned from the committee"]
                    BanValidators(
                        ::std::vec::Vec<(
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            runtime_types::primitives::BanInfo,
                        )>,
                    ),
                }
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ValidatorTotalRewards<_0>(
                pub ::subxt::utils::KeyedVec<_0, ::core::primitive::u32>,
            );
        }
        pub mod pallet_identity {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Identity pallet declaration."]
                pub enum Call {
                    #[codec(index = 0)]
                    #[doc = "Add a registrar to the system."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be `T::RegistrarOrigin`."]
                    #[doc = ""]
                    #[doc = "- `account`: the account of the registrar."]
                    #[doc = ""]
                    #[doc = "Emits `RegistrarAdded` if successful."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(R)` where `R` registrar-count (governance-bounded and code-bounded)."]
                    #[doc = "- One storage mutation (codec `O(R)`)."]
                    #[doc = "- One event."]
                    #[doc = "# </weight>"]
                    add_registrar {
                        account: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                    },
                    #[codec(index = 1)]
                    #[doc = "Set an account's identity information and reserve the appropriate deposit."]
                    #[doc = ""]
                    #[doc = "If the account already has identity information, the deposit is taken as part payment"]
                    #[doc = "for the new deposit."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_."]
                    #[doc = ""]
                    #[doc = "- `info`: The identity information."]
                    #[doc = ""]
                    #[doc = "Emits `IdentitySet` if successful."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(X + X' + R)`"]
                    #[doc = "  - where `X` additional-field-count (deposit-bounded and code-bounded)"]
                    #[doc = "  - where `R` judgements-count (registrar-count-bounded)"]
                    #[doc = "- One balance reserve operation."]
                    #[doc = "- One storage mutation (codec-read `O(X' + R)`, codec-write `O(X + R)`)."]
                    #[doc = "- One event."]
                    #[doc = "# </weight>"]
                    set_identity {
                        info:
                            ::std::boxed::Box<runtime_types::pallet_identity::types::IdentityInfo>,
                    },
                    #[codec(index = 2)]
                    #[doc = "Set the sub-accounts of the sender."]
                    #[doc = ""]
                    #[doc = "Payment: Any aggregate balance reserved by previous `set_subs` calls will be returned"]
                    #[doc = "and an amount `SubAccountDeposit` will be reserved for each item in `subs`."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_ and the sender must have a registered"]
                    #[doc = "identity."]
                    #[doc = ""]
                    #[doc = "- `subs`: The identity's (new) sub-accounts."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(P + S)`"]
                    #[doc = "  - where `P` old-subs-count (hard- and deposit-bounded)."]
                    #[doc = "  - where `S` subs-count (hard- and deposit-bounded)."]
                    #[doc = "- At most one balance operations."]
                    #[doc = "- DB:"]
                    #[doc = "  - `P + S` storage mutations (codec complexity `O(1)`)"]
                    #[doc = "  - One storage read (codec complexity `O(P)`)."]
                    #[doc = "  - One storage write (codec complexity `O(S)`)."]
                    #[doc = "  - One storage-exists (`IdentityOf::contains_key`)."]
                    #[doc = "# </weight>"]
                    set_subs {
                        subs: ::std::vec::Vec<(
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            runtime_types::pallet_identity::types::Data,
                        )>,
                    },
                    #[codec(index = 3)]
                    #[doc = "Clear an account's identity info and all sub-accounts and return all deposits."]
                    #[doc = ""]
                    #[doc = "Payment: All reserved balances on the account are returned."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_ and the sender must have a registered"]
                    #[doc = "identity."]
                    #[doc = ""]
                    #[doc = "Emits `IdentityCleared` if successful."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(R + S + X)`"]
                    #[doc = "  - where `R` registrar-count (governance-bounded)."]
                    #[doc = "  - where `S` subs-count (hard- and deposit-bounded)."]
                    #[doc = "  - where `X` additional-field-count (deposit-bounded and code-bounded)."]
                    #[doc = "- One balance-unreserve operation."]
                    #[doc = "- `2` storage reads and `S + 2` storage deletions."]
                    #[doc = "- One event."]
                    #[doc = "# </weight>"]
                    clear_identity,
                    #[codec(index = 4)]
                    #[doc = "Request a judgement from a registrar."]
                    #[doc = ""]
                    #[doc = "Payment: At most `max_fee` will be reserved for payment to the registrar if judgement"]
                    #[doc = "given."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_ and the sender must have a"]
                    #[doc = "registered identity."]
                    #[doc = ""]
                    #[doc = "- `reg_index`: The index of the registrar whose judgement is requested."]
                    #[doc = "- `max_fee`: The maximum fee that may be paid. This should just be auto-populated as:"]
                    #[doc = ""]
                    #[doc = "```nocompile"]
                    #[doc = "Self::registrars().get(reg_index).unwrap().fee"]
                    #[doc = "```"]
                    #[doc = ""]
                    #[doc = "Emits `JudgementRequested` if successful."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(R + X)`."]
                    #[doc = "- One balance-reserve operation."]
                    #[doc = "- Storage: 1 read `O(R)`, 1 mutate `O(X + R)`."]
                    #[doc = "- One event."]
                    #[doc = "# </weight>"]
                    request_judgement {
                        #[codec(compact)]
                        reg_index: ::core::primitive::u32,
                        #[codec(compact)]
                        max_fee: ::core::primitive::u128,
                    },
                    #[codec(index = 5)]
                    #[doc = "Cancel a previous request."]
                    #[doc = ""]
                    #[doc = "Payment: A previously reserved deposit is returned on success."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_ and the sender must have a"]
                    #[doc = "registered identity."]
                    #[doc = ""]
                    #[doc = "- `reg_index`: The index of the registrar whose judgement is no longer requested."]
                    #[doc = ""]
                    #[doc = "Emits `JudgementUnrequested` if successful."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(R + X)`."]
                    #[doc = "- One balance-reserve operation."]
                    #[doc = "- One storage mutation `O(R + X)`."]
                    #[doc = "- One event"]
                    #[doc = "# </weight>"]
                    cancel_request { reg_index: ::core::primitive::u32 },
                    #[codec(index = 6)]
                    #[doc = "Set the fee required for a judgement to be requested from a registrar."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_ and the sender must be the account"]
                    #[doc = "of the registrar whose index is `index`."]
                    #[doc = ""]
                    #[doc = "- `index`: the index of the registrar whose fee is to be set."]
                    #[doc = "- `fee`: the new fee."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(R)`."]
                    #[doc = "- One storage mutation `O(R)`."]
                    #[doc = "- Benchmark: 7.315 + R * 0.329 µs (min squares analysis)"]
                    #[doc = "# </weight>"]
                    set_fee {
                        #[codec(compact)]
                        index: ::core::primitive::u32,
                        #[codec(compact)]
                        fee: ::core::primitive::u128,
                    },
                    #[codec(index = 7)]
                    #[doc = "Change the account associated with a registrar."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_ and the sender must be the account"]
                    #[doc = "of the registrar whose index is `index`."]
                    #[doc = ""]
                    #[doc = "- `index`: the index of the registrar whose fee is to be set."]
                    #[doc = "- `new`: the new account ID."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(R)`."]
                    #[doc = "- One storage mutation `O(R)`."]
                    #[doc = "- Benchmark: 8.823 + R * 0.32 µs (min squares analysis)"]
                    #[doc = "# </weight>"]
                    set_account_id {
                        #[codec(compact)]
                        index: ::core::primitive::u32,
                        new: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                    },
                    #[codec(index = 8)]
                    #[doc = "Set the field information for a registrar."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_ and the sender must be the account"]
                    #[doc = "of the registrar whose index is `index`."]
                    #[doc = ""]
                    #[doc = "- `index`: the index of the registrar whose fee is to be set."]
                    #[doc = "- `fields`: the fields that the registrar concerns themselves with."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(R)`."]
                    #[doc = "- One storage mutation `O(R)`."]
                    #[doc = "- Benchmark: 7.464 + R * 0.325 µs (min squares analysis)"]
                    #[doc = "# </weight>"]
                    set_fields {
                        #[codec(compact)]
                        index: ::core::primitive::u32,
                        fields: runtime_types::pallet_identity::types::BitFlags<
                            runtime_types::pallet_identity::types::IdentityField,
                        >,
                    },
                    #[codec(index = 9)]
                    #[doc = "Provide a judgement for an account's identity."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_ and the sender must be the account"]
                    #[doc = "of the registrar whose index is `reg_index`."]
                    #[doc = ""]
                    #[doc = "- `reg_index`: the index of the registrar whose judgement is being made."]
                    #[doc = "- `target`: the account whose identity the judgement is upon. This must be an account"]
                    #[doc = "  with a registered identity."]
                    #[doc = "- `judgement`: the judgement of the registrar of index `reg_index` about `target`."]
                    #[doc = "- `identity`: The hash of the [`IdentityInfo`] for that the judgement is provided."]
                    #[doc = ""]
                    #[doc = "Emits `JudgementGiven` if successful."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(R + X)`."]
                    #[doc = "- One balance-transfer operation."]
                    #[doc = "- Up to one account-lookup operation."]
                    #[doc = "- Storage: 1 read `O(R)`, 1 mutate `O(R + X)`."]
                    #[doc = "- One event."]
                    #[doc = "# </weight>"]
                    provide_judgement {
                        #[codec(compact)]
                        reg_index: ::core::primitive::u32,
                        target: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        judgement: runtime_types::pallet_identity::types::Judgement<
                            ::core::primitive::u128,
                        >,
                        identity: ::subxt::ext::sp_core::H256,
                    },
                    #[codec(index = 10)]
                    #[doc = "Remove an account's identity and sub-account information and slash the deposits."]
                    #[doc = ""]
                    #[doc = "Payment: Reserved balances from `set_subs` and `set_identity` are slashed and handled by"]
                    #[doc = "`Slash`. Verification request deposits are not returned; they should be cancelled"]
                    #[doc = "manually using `cancel_request`."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must match `T::ForceOrigin`."]
                    #[doc = ""]
                    #[doc = "- `target`: the account whose identity the judgement is upon. This must be an account"]
                    #[doc = "  with a registered identity."]
                    #[doc = ""]
                    #[doc = "Emits `IdentityKilled` if successful."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(R + S + X)`."]
                    #[doc = "- One balance-reserve operation."]
                    #[doc = "- `S + 2` storage mutations."]
                    #[doc = "- One event."]
                    #[doc = "# </weight>"]
                    kill_identity {
                        target: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                    },
                    #[codec(index = 11)]
                    #[doc = "Add the given account to the sender's subs."]
                    #[doc = ""]
                    #[doc = "Payment: Balance reserved by a previous `set_subs` call for one sub will be repatriated"]
                    #[doc = "to the sender."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_ and the sender must have a registered"]
                    #[doc = "sub identity of `sub`."]
                    add_sub {
                        sub: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        data: runtime_types::pallet_identity::types::Data,
                    },
                    #[codec(index = 12)]
                    #[doc = "Alter the associated name of the given sub-account."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_ and the sender must have a registered"]
                    #[doc = "sub identity of `sub`."]
                    rename_sub {
                        sub: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        data: runtime_types::pallet_identity::types::Data,
                    },
                    #[codec(index = 13)]
                    #[doc = "Remove the given account from the sender's subs."]
                    #[doc = ""]
                    #[doc = "Payment: Balance reserved by a previous `set_subs` call for one sub will be repatriated"]
                    #[doc = "to the sender."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_ and the sender must have a registered"]
                    #[doc = "sub identity of `sub`."]
                    remove_sub {
                        sub: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                    },
                    #[codec(index = 14)]
                    #[doc = "Remove the sender as a sub-account."]
                    #[doc = ""]
                    #[doc = "Payment: Balance reserved by a previous `set_subs` call for one sub will be repatriated"]
                    #[doc = "to the sender (*not* the original depositor)."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_ and the sender must have a registered"]
                    #[doc = "super-identity."]
                    #[doc = ""]
                    #[doc = "NOTE: This should not normally be used, but is provided in the case that the non-"]
                    #[doc = "controller of an account is maliciously registered as a sub-account."]
                    quit_sub,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tCustom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)\n\t\t\tof this pallet.\n\t\t\t"]
                pub enum Error {
                    #[codec(index = 0)]
                    #[doc = "Too many subs-accounts."]
                    TooManySubAccounts,
                    #[codec(index = 1)]
                    #[doc = "Account isn't found."]
                    NotFound,
                    #[codec(index = 2)]
                    #[doc = "Account isn't named."]
                    NotNamed,
                    #[codec(index = 3)]
                    #[doc = "Empty index."]
                    EmptyIndex,
                    #[codec(index = 4)]
                    #[doc = "Fee is changed."]
                    FeeChanged,
                    #[codec(index = 5)]
                    #[doc = "No identity found."]
                    NoIdentity,
                    #[codec(index = 6)]
                    #[doc = "Sticky judgement."]
                    StickyJudgement,
                    #[codec(index = 7)]
                    #[doc = "Judgement given."]
                    JudgementGiven,
                    #[codec(index = 8)]
                    #[doc = "Invalid judgement."]
                    InvalidJudgement,
                    #[codec(index = 9)]
                    #[doc = "The index is invalid."]
                    InvalidIndex,
                    #[codec(index = 10)]
                    #[doc = "The target is invalid."]
                    InvalidTarget,
                    #[codec(index = 11)]
                    #[doc = "Too many additional fields."]
                    TooManyFields,
                    #[codec(index = 12)]
                    #[doc = "Maximum amount of registrars reached. Cannot add any more."]
                    TooManyRegistrars,
                    #[codec(index = 13)]
                    #[doc = "Account ID is already named."]
                    AlreadyClaimed,
                    #[codec(index = 14)]
                    #[doc = "Sender is not a sub-account."]
                    NotSub,
                    #[codec(index = 15)]
                    #[doc = "Sub-account isn't owned by sender."]
                    NotOwned,
                    #[codec(index = 16)]
                    #[doc = "The provided judgement was for a different identity."]
                    JudgementForDifferentIdentity,
                    #[codec(index = 17)]
                    #[doc = "Error that occurs when there is an issue paying for judgement."]
                    JudgementPaymentFailed,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
                pub enum Event {
                    #[codec(index = 0)]
                    #[doc = "A name was set or reset (which will remove all judgements)."]
                    IdentitySet {
                        who: ::subxt::ext::sp_core::crypto::AccountId32,
                    },
                    #[codec(index = 1)]
                    #[doc = "A name was cleared, and the given balance returned."]
                    IdentityCleared {
                        who: ::subxt::ext::sp_core::crypto::AccountId32,
                        deposit: ::core::primitive::u128,
                    },
                    #[codec(index = 2)]
                    #[doc = "A name was removed and the given balance slashed."]
                    IdentityKilled {
                        who: ::subxt::ext::sp_core::crypto::AccountId32,
                        deposit: ::core::primitive::u128,
                    },
                    #[codec(index = 3)]
                    #[doc = "A judgement was asked from a registrar."]
                    JudgementRequested {
                        who: ::subxt::ext::sp_core::crypto::AccountId32,
                        registrar_index: ::core::primitive::u32,
                    },
                    #[codec(index = 4)]
                    #[doc = "A judgement request was retracted."]
                    JudgementUnrequested {
                        who: ::subxt::ext::sp_core::crypto::AccountId32,
                        registrar_index: ::core::primitive::u32,
                    },
                    #[codec(index = 5)]
                    #[doc = "A judgement was given by a registrar."]
                    JudgementGiven {
                        target: ::subxt::ext::sp_core::crypto::AccountId32,
                        registrar_index: ::core::primitive::u32,
                    },
                    #[codec(index = 6)]
                    #[doc = "A registrar was added."]
                    RegistrarAdded {
                        registrar_index: ::core::primitive::u32,
                    },
                    #[codec(index = 7)]
                    #[doc = "A sub-identity was added to an identity and the deposit paid."]
                    SubIdentityAdded {
                        sub: ::subxt::ext::sp_core::crypto::AccountId32,
                        main: ::subxt::ext::sp_core::crypto::AccountId32,
                        deposit: ::core::primitive::u128,
                    },
                    #[codec(index = 8)]
                    #[doc = "A sub-identity was removed from an identity and the deposit freed."]
                    SubIdentityRemoved {
                        sub: ::subxt::ext::sp_core::crypto::AccountId32,
                        main: ::subxt::ext::sp_core::crypto::AccountId32,
                        deposit: ::core::primitive::u128,
                    },
                    #[codec(index = 9)]
                    #[doc = "A sub-identity was cleared, and the given deposit repatriated from the"]
                    #[doc = "main identity account to the sub-identity account."]
                    SubIdentityRevoked {
                        sub: ::subxt::ext::sp_core::crypto::AccountId32,
                        main: ::subxt::ext::sp_core::crypto::AccountId32,
                        deposit: ::core::primitive::u128,
                    },
                }
            }
            pub mod types {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: CompactAs,
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct BitFlags<_0>(
                    pub ::core::primitive::u64,
                    #[codec(skip)] pub ::core::marker::PhantomData<_0>,
                );
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub enum Data {
                    #[codec(index = 0)]
                    None,
                    #[codec(index = 1)]
                    Raw0([::core::primitive::u8; 0usize]),
                    #[codec(index = 2)]
                    Raw1([::core::primitive::u8; 1usize]),
                    #[codec(index = 3)]
                    Raw2([::core::primitive::u8; 2usize]),
                    #[codec(index = 4)]
                    Raw3([::core::primitive::u8; 3usize]),
                    #[codec(index = 5)]
                    Raw4([::core::primitive::u8; 4usize]),
                    #[codec(index = 6)]
                    Raw5([::core::primitive::u8; 5usize]),
                    #[codec(index = 7)]
                    Raw6([::core::primitive::u8; 6usize]),
                    #[codec(index = 8)]
                    Raw7([::core::primitive::u8; 7usize]),
                    #[codec(index = 9)]
                    Raw8([::core::primitive::u8; 8usize]),
                    #[codec(index = 10)]
                    Raw9([::core::primitive::u8; 9usize]),
                    #[codec(index = 11)]
                    Raw10([::core::primitive::u8; 10usize]),
                    #[codec(index = 12)]
                    Raw11([::core::primitive::u8; 11usize]),
                    #[codec(index = 13)]
                    Raw12([::core::primitive::u8; 12usize]),
                    #[codec(index = 14)]
                    Raw13([::core::primitive::u8; 13usize]),
                    #[codec(index = 15)]
                    Raw14([::core::primitive::u8; 14usize]),
                    #[codec(index = 16)]
                    Raw15([::core::primitive::u8; 15usize]),
                    #[codec(index = 17)]
                    Raw16([::core::primitive::u8; 16usize]),
                    #[codec(index = 18)]
                    Raw17([::core::primitive::u8; 17usize]),
                    #[codec(index = 19)]
                    Raw18([::core::primitive::u8; 18usize]),
                    #[codec(index = 20)]
                    Raw19([::core::primitive::u8; 19usize]),
                    #[codec(index = 21)]
                    Raw20([::core::primitive::u8; 20usize]),
                    #[codec(index = 22)]
                    Raw21([::core::primitive::u8; 21usize]),
                    #[codec(index = 23)]
                    Raw22([::core::primitive::u8; 22usize]),
                    #[codec(index = 24)]
                    Raw23([::core::primitive::u8; 23usize]),
                    #[codec(index = 25)]
                    Raw24([::core::primitive::u8; 24usize]),
                    #[codec(index = 26)]
                    Raw25([::core::primitive::u8; 25usize]),
                    #[codec(index = 27)]
                    Raw26([::core::primitive::u8; 26usize]),
                    #[codec(index = 28)]
                    Raw27([::core::primitive::u8; 27usize]),
                    #[codec(index = 29)]
                    Raw28([::core::primitive::u8; 28usize]),
                    #[codec(index = 30)]
                    Raw29([::core::primitive::u8; 29usize]),
                    #[codec(index = 31)]
                    Raw30([::core::primitive::u8; 30usize]),
                    #[codec(index = 32)]
                    Raw31([::core::primitive::u8; 31usize]),
                    #[codec(index = 33)]
                    Raw32([::core::primitive::u8; 32usize]),
                    #[codec(index = 34)]
                    BlakeTwo256([::core::primitive::u8; 32usize]),
                    #[codec(index = 35)]
                    Sha256([::core::primitive::u8; 32usize]),
                    #[codec(index = 36)]
                    Keccak256([::core::primitive::u8; 32usize]),
                    #[codec(index = 37)]
                    ShaThree256([::core::primitive::u8; 32usize]),
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub enum IdentityField {
                    #[codec(index = 1)]
                    Display,
                    #[codec(index = 2)]
                    Legal,
                    #[codec(index = 4)]
                    Web,
                    #[codec(index = 8)]
                    Riot,
                    #[codec(index = 16)]
                    Email,
                    #[codec(index = 32)]
                    PgpFingerprint,
                    #[codec(index = 64)]
                    Image,
                    #[codec(index = 128)]
                    Twitter,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct IdentityInfo {
                    pub additional: runtime_types::sp_core::bounded::bounded_vec::BoundedVec<(
                        runtime_types::pallet_identity::types::Data,
                        runtime_types::pallet_identity::types::Data,
                    )>,
                    pub display: runtime_types::pallet_identity::types::Data,
                    pub legal: runtime_types::pallet_identity::types::Data,
                    pub web: runtime_types::pallet_identity::types::Data,
                    pub riot: runtime_types::pallet_identity::types::Data,
                    pub email: runtime_types::pallet_identity::types::Data,
                    pub pgp_fingerprint: ::core::option::Option<[::core::primitive::u8; 20usize]>,
                    pub image: runtime_types::pallet_identity::types::Data,
                    pub twitter: runtime_types::pallet_identity::types::Data,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub enum Judgement<_0> {
                    #[codec(index = 0)]
                    Unknown,
                    #[codec(index = 1)]
                    FeePaid(_0),
                    #[codec(index = 2)]
                    Reasonable,
                    #[codec(index = 3)]
                    KnownGood,
                    #[codec(index = 4)]
                    OutOfDate,
                    #[codec(index = 5)]
                    LowQuality,
                    #[codec(index = 6)]
                    Erroneous,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct RegistrarInfo<_0, _1> {
                    pub account: _1,
                    pub fee: _0,
                    pub fields: runtime_types::pallet_identity::types::BitFlags<
                        runtime_types::pallet_identity::types::IdentityField,
                    >,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct Registration<_0> {
                    pub judgements: runtime_types::sp_core::bounded::bounded_vec::BoundedVec<(
                        ::core::primitive::u32,
                        runtime_types::pallet_identity::types::Judgement<_0>,
                    )>,
                    pub deposit: _0,
                    pub info: runtime_types::pallet_identity::types::IdentityInfo,
                }
            }
        }
        pub mod pallet_multisig {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
                pub enum Call {
                    #[codec(index = 0)]
                    #[doc = "Immediately dispatch a multi-signature call using a single approval from the caller."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_."]
                    #[doc = ""]
                    #[doc = "- `other_signatories`: The accounts (other than the sender) who are part of the"]
                    #[doc = "multi-signature, but do not participate in the approval process."]
                    #[doc = "- `call`: The call to be executed."]
                    #[doc = ""]
                    #[doc = "Result is equivalent to the dispatched result."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "O(Z + C) where Z is the length of the call and C its execution weight."]
                    #[doc = "-------------------------------"]
                    #[doc = "- DB Weight: None"]
                    #[doc = "- Plus Call Weight"]
                    #[doc = "# </weight>"]
                    as_multi_threshold_1 {
                        other_signatories:
                            ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                        call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
                    },
                    #[codec(index = 1)]
                    #[doc = "Register approval for a dispatch to be made from a deterministic composite account if"]
                    #[doc = "approved by a total of `threshold - 1` of `other_signatories`."]
                    #[doc = ""]
                    #[doc = "If there are enough, then dispatch the call."]
                    #[doc = ""]
                    #[doc = "Payment: `DepositBase` will be reserved if this is the first approval, plus"]
                    #[doc = "`threshold` times `DepositFactor`. It is returned once this dispatch happens or"]
                    #[doc = "is cancelled."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_."]
                    #[doc = ""]
                    #[doc = "- `threshold`: The total number of approvals for this dispatch before it is executed."]
                    #[doc = "- `other_signatories`: The accounts (other than the sender) who can approve this"]
                    #[doc = "dispatch. May not be empty."]
                    #[doc = "- `maybe_timepoint`: If this is the first approval, then this must be `None`. If it is"]
                    #[doc = "not the first approval, then it must be `Some`, with the timepoint (block number and"]
                    #[doc = "transaction index) of the first approval transaction."]
                    #[doc = "- `call`: The call to be executed."]
                    #[doc = ""]
                    #[doc = "NOTE: Unless this is the final approval, you will generally want to use"]
                    #[doc = "`approve_as_multi` instead, since it only requires a hash of the call."]
                    #[doc = ""]
                    #[doc = "Result is equivalent to the dispatched result if `threshold` is exactly `1`. Otherwise"]
                    #[doc = "on success, result is `Ok` and the result from the interior call, if it was executed,"]
                    #[doc = "may be found in the deposited `MultisigExecuted` event."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(S + Z + Call)`."]
                    #[doc = "- Up to one balance-reserve or unreserve operation."]
                    #[doc = "- One passthrough operation, one insert, both `O(S)` where `S` is the number of"]
                    #[doc = "  signatories. `S` is capped by `MaxSignatories`, with weight being proportional."]
                    #[doc = "- One call encode & hash, both of complexity `O(Z)` where `Z` is tx-len."]
                    #[doc = "- One encode & hash, both of complexity `O(S)`."]
                    #[doc = "- Up to one binary search and insert (`O(logS + S)`)."]
                    #[doc = "- I/O: 1 read `O(S)`, up to 1 mutate `O(S)`. Up to one remove."]
                    #[doc = "- One event."]
                    #[doc = "- The weight of the `call`."]
                    #[doc = "- Storage: inserts one item, value size bounded by `MaxSignatories`, with a deposit"]
                    #[doc = "  taken for its lifetime of `DepositBase + threshold * DepositFactor`."]
                    #[doc = "-------------------------------"]
                    #[doc = "- DB Weight:"]
                    #[doc = "    - Reads: Multisig Storage, [Caller Account]"]
                    #[doc = "    - Writes: Multisig Storage, [Caller Account]"]
                    #[doc = "- Plus Call Weight"]
                    #[doc = "# </weight>"]
                    as_multi {
                        threshold: ::core::primitive::u16,
                        other_signatories:
                            ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                        maybe_timepoint: ::core::option::Option<
                            runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
                        >,
                        call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
                        max_weight: runtime_types::sp_weights::weight_v2::Weight,
                    },
                    #[codec(index = 2)]
                    #[doc = "Register approval for a dispatch to be made from a deterministic composite account if"]
                    #[doc = "approved by a total of `threshold - 1` of `other_signatories`."]
                    #[doc = ""]
                    #[doc = "Payment: `DepositBase` will be reserved if this is the first approval, plus"]
                    #[doc = "`threshold` times `DepositFactor`. It is returned once this dispatch happens or"]
                    #[doc = "is cancelled."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_."]
                    #[doc = ""]
                    #[doc = "- `threshold`: The total number of approvals for this dispatch before it is executed."]
                    #[doc = "- `other_signatories`: The accounts (other than the sender) who can approve this"]
                    #[doc = "dispatch. May not be empty."]
                    #[doc = "- `maybe_timepoint`: If this is the first approval, then this must be `None`. If it is"]
                    #[doc = "not the first approval, then it must be `Some`, with the timepoint (block number and"]
                    #[doc = "transaction index) of the first approval transaction."]
                    #[doc = "- `call_hash`: The hash of the call to be executed."]
                    #[doc = ""]
                    #[doc = "NOTE: If this is the final approval, you will want to use `as_multi` instead."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(S)`."]
                    #[doc = "- Up to one balance-reserve or unreserve operation."]
                    #[doc = "- One passthrough operation, one insert, both `O(S)` where `S` is the number of"]
                    #[doc = "  signatories. `S` is capped by `MaxSignatories`, with weight being proportional."]
                    #[doc = "- One encode & hash, both of complexity `O(S)`."]
                    #[doc = "- Up to one binary search and insert (`O(logS + S)`)."]
                    #[doc = "- I/O: 1 read `O(S)`, up to 1 mutate `O(S)`. Up to one remove."]
                    #[doc = "- One event."]
                    #[doc = "- Storage: inserts one item, value size bounded by `MaxSignatories`, with a deposit"]
                    #[doc = "  taken for its lifetime of `DepositBase + threshold * DepositFactor`."]
                    #[doc = "----------------------------------"]
                    #[doc = "- DB Weight:"]
                    #[doc = "    - Read: Multisig Storage, [Caller Account]"]
                    #[doc = "    - Write: Multisig Storage, [Caller Account]"]
                    #[doc = "# </weight>"]
                    approve_as_multi {
                        threshold: ::core::primitive::u16,
                        other_signatories:
                            ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                        maybe_timepoint: ::core::option::Option<
                            runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
                        >,
                        call_hash: [::core::primitive::u8; 32usize],
                        max_weight: runtime_types::sp_weights::weight_v2::Weight,
                    },
                    #[codec(index = 3)]
                    #[doc = "Cancel a pre-existing, on-going multisig transaction. Any deposit reserved previously"]
                    #[doc = "for this operation will be unreserved on success."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_."]
                    #[doc = ""]
                    #[doc = "- `threshold`: The total number of approvals for this dispatch before it is executed."]
                    #[doc = "- `other_signatories`: The accounts (other than the sender) who can approve this"]
                    #[doc = "dispatch. May not be empty."]
                    #[doc = "- `timepoint`: The timepoint (block number and transaction index) of the first approval"]
                    #[doc = "transaction for this dispatch."]
                    #[doc = "- `call_hash`: The hash of the call to be executed."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(S)`."]
                    #[doc = "- Up to one balance-reserve or unreserve operation."]
                    #[doc = "- One passthrough operation, one insert, both `O(S)` where `S` is the number of"]
                    #[doc = "  signatories. `S` is capped by `MaxSignatories`, with weight being proportional."]
                    #[doc = "- One encode & hash, both of complexity `O(S)`."]
                    #[doc = "- One event."]
                    #[doc = "- I/O: 1 read `O(S)`, one remove."]
                    #[doc = "- Storage: removes one item."]
                    #[doc = "----------------------------------"]
                    #[doc = "- DB Weight:"]
                    #[doc = "    - Read: Multisig Storage, [Caller Account], Refund Account"]
                    #[doc = "    - Write: Multisig Storage, [Caller Account], Refund Account"]
                    #[doc = "# </weight>"]
                    cancel_as_multi {
                        threshold: ::core::primitive::u16,
                        other_signatories:
                            ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                        timepoint:
                            runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
                        call_hash: [::core::primitive::u8; 32usize],
                    },
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tCustom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)\n\t\t\tof this pallet.\n\t\t\t"]
                pub enum Error {
                    #[codec(index = 0)]
                    #[doc = "Threshold must be 2 or greater."]
                    MinimumThreshold,
                    #[codec(index = 1)]
                    #[doc = "Call is already approved by this signatory."]
                    AlreadyApproved,
                    #[codec(index = 2)]
                    #[doc = "Call doesn't need any (more) approvals."]
                    NoApprovalsNeeded,
                    #[codec(index = 3)]
                    #[doc = "There are too few signatories in the list."]
                    TooFewSignatories,
                    #[codec(index = 4)]
                    #[doc = "There are too many signatories in the list."]
                    TooManySignatories,
                    #[codec(index = 5)]
                    #[doc = "The signatories were provided out of order; they should be ordered."]
                    SignatoriesOutOfOrder,
                    #[codec(index = 6)]
                    #[doc = "The sender was contained in the other signatories; it shouldn't be."]
                    SenderInSignatories,
                    #[codec(index = 7)]
                    #[doc = "Multisig operation not found when attempting to cancel."]
                    NotFound,
                    #[codec(index = 8)]
                    #[doc = "Only the account that originally created the multisig is able to cancel it."]
                    NotOwner,
                    #[codec(index = 9)]
                    #[doc = "No timepoint was given, yet the multisig operation is already underway."]
                    NoTimepoint,
                    #[codec(index = 10)]
                    #[doc = "A different timepoint was given to the multisig operation that is underway."]
                    WrongTimepoint,
                    #[codec(index = 11)]
                    #[doc = "A timepoint was given, yet no multisig operation is underway."]
                    UnexpectedTimepoint,
                    #[codec(index = 12)]
                    #[doc = "The maximum weight information provided was too low."]
                    MaxWeightTooLow,
                    #[codec(index = 13)]
                    #[doc = "The data to be stored is already stored."]
                    AlreadyStored,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
                pub enum Event {
                    #[codec(index = 0)]
                    #[doc = "A new multisig operation has begun."]
                    NewMultisig {
                        approving: ::subxt::ext::sp_core::crypto::AccountId32,
                        multisig: ::subxt::ext::sp_core::crypto::AccountId32,
                        call_hash: [::core::primitive::u8; 32usize],
                    },
                    #[codec(index = 1)]
                    #[doc = "A multisig operation has been approved by someone."]
                    MultisigApproval {
                        approving: ::subxt::ext::sp_core::crypto::AccountId32,
                        timepoint:
                            runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
                        multisig: ::subxt::ext::sp_core::crypto::AccountId32,
                        call_hash: [::core::primitive::u8; 32usize],
                    },
                    #[codec(index = 2)]
                    #[doc = "A multisig operation has been executed."]
                    MultisigExecuted {
                        approving: ::subxt::ext::sp_core::crypto::AccountId32,
                        timepoint:
                            runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
                        multisig: ::subxt::ext::sp_core::crypto::AccountId32,
                        call_hash: [::core::primitive::u8; 32usize],
                        result:
                            ::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
                    },
                    #[codec(index = 3)]
                    #[doc = "A multisig operation has been cancelled."]
                    MultisigCancelled {
                        cancelling: ::subxt::ext::sp_core::crypto::AccountId32,
                        timepoint:
                            runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
                        multisig: ::subxt::ext::sp_core::crypto::AccountId32,
                        call_hash: [::core::primitive::u8; 32usize],
                    },
                }
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Multisig<_0, _1, _2> {
                pub when: runtime_types::pallet_multisig::Timepoint<_0>,
                pub deposit: _1,
                pub depositor: _2,
                pub approvals: runtime_types::sp_core::bounded::bounded_vec::BoundedVec<_2>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Timepoint<_0> {
                pub height: _0,
                pub index: _0,
            }
        }
        pub mod pallet_nomination_pools {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
                pub enum Call {
                    #[codec(index = 0)]
                    #[doc = "Stake funds with a pool. The amount to bond is transferred from the member to the"]
                    #[doc = "pools account and immediately increases the pools bond."]
                    #[doc = ""]
                    #[doc = "# Note"]
                    #[doc = ""]
                    #[doc = "* An account can only be a member of a single pool."]
                    #[doc = "* An account cannot join the same pool multiple times."]
                    #[doc = "* This call will *not* dust the member account, so the member must have at least"]
                    #[doc = "  `existential deposit + amount` in their account."]
                    #[doc = "* Only a pool with [`PoolState::Open`] can be joined"]
                    join {
                        #[codec(compact)]
                        amount: ::core::primitive::u128,
                        pool_id: ::core::primitive::u32,
                    },
                    #[codec(index = 1)]
                    #[doc = "Bond `extra` more funds from `origin` into the pool to which they already belong."]
                    #[doc = ""]
                    #[doc = "Additional funds can come from either the free balance of the account, of from the"]
                    #[doc = "accumulated rewards, see [`BondExtra`]."]
                    #[doc = ""]
                    #[doc = "Bonding extra funds implies an automatic payout of all pending rewards as well."]
                    bond_extra {
                        extra: runtime_types::pallet_nomination_pools::BondExtra<
                            ::core::primitive::u128,
                        >,
                    },
                    #[codec(index = 2)]
                    #[doc = "A bonded member can use this to claim their payout based on the rewards that the pool"]
                    #[doc = "has accumulated since their last claimed payout (OR since joining if this is there first"]
                    #[doc = "time claiming rewards). The payout will be transferred to the member's account."]
                    #[doc = ""]
                    #[doc = "The member will earn rewards pro rata based on the members stake vs the sum of the"]
                    #[doc = "members in the pools stake. Rewards do not \"expire\"."]
                    claim_payout,
                    #[codec(index = 3)]
                    #[doc = "Unbond up to `unbonding_points` of the `member_account`'s funds from the pool. It"]
                    #[doc = "implicitly collects the rewards one last time, since not doing so would mean some"]
                    #[doc = "rewards would be forfeited."]
                    #[doc = ""]
                    #[doc = "Under certain conditions, this call can be dispatched permissionlessly (i.e. by any"]
                    #[doc = "account)."]
                    #[doc = ""]
                    #[doc = "# Conditions for a permissionless dispatch."]
                    #[doc = ""]
                    #[doc = "* The pool is blocked and the caller is either the root or state-toggler. This is"]
                    #[doc = "  refereed to as a kick."]
                    #[doc = "* The pool is destroying and the member is not the depositor."]
                    #[doc = "* The pool is destroying, the member is the depositor and no other members are in the"]
                    #[doc = "  pool."]
                    #[doc = ""]
                    #[doc = "## Conditions for permissioned dispatch (i.e. the caller is also the"]
                    #[doc = "`member_account`):"]
                    #[doc = ""]
                    #[doc = "* The caller is not the depositor."]
                    #[doc = "* The caller is the depositor, the pool is destroying and no other members are in the"]
                    #[doc = "  pool."]
                    #[doc = ""]
                    #[doc = "# Note"]
                    #[doc = ""]
                    #[doc = "If there are too many unlocking chunks to unbond with the pool account,"]
                    #[doc = "[`Call::pool_withdraw_unbonded`] can be called to try and minimize unlocking chunks."]
                    #[doc = "The [`StakingInterface::unbond`] will implicitly call [`Call::pool_withdraw_unbonded`]"]
                    #[doc = "to try to free chunks if necessary (ie. if unbound was called and no unlocking chunks"]
                    #[doc = "are available). However, it may not be possible to release the current unlocking chunks,"]
                    #[doc = "in which case, the result of this call will likely be the `NoMoreChunks` error from the"]
                    #[doc = "staking system."]
                    unbond {
                        member_account: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        #[codec(compact)]
                        unbonding_points: ::core::primitive::u128,
                    },
                    #[codec(index = 4)]
                    #[doc = "Call `withdraw_unbonded` for the pools account. This call can be made by any account."]
                    #[doc = ""]
                    #[doc = "This is useful if their are too many unlocking chunks to call `unbond`, and some"]
                    #[doc = "can be cleared by withdrawing. In the case there are too many unlocking chunks, the user"]
                    #[doc = "would probably see an error like `NoMoreChunks` emitted from the staking system when"]
                    #[doc = "they attempt to unbond."]
                    pool_withdraw_unbonded {
                        pool_id: ::core::primitive::u32,
                        num_slashing_spans: ::core::primitive::u32,
                    },
                    #[codec(index = 5)]
                    #[doc = "Withdraw unbonded funds from `member_account`. If no bonded funds can be unbonded, an"]
                    #[doc = "error is returned."]
                    #[doc = ""]
                    #[doc = "Under certain conditions, this call can be dispatched permissionlessly (i.e. by any"]
                    #[doc = "account)."]
                    #[doc = ""]
                    #[doc = "# Conditions for a permissionless dispatch"]
                    #[doc = ""]
                    #[doc = "* The pool is in destroy mode and the target is not the depositor."]
                    #[doc = "* The target is the depositor and they are the only member in the sub pools."]
                    #[doc = "* The pool is blocked and the caller is either the root or state-toggler."]
                    #[doc = ""]
                    #[doc = "# Conditions for permissioned dispatch"]
                    #[doc = ""]
                    #[doc = "* The caller is the target and they are not the depositor."]
                    #[doc = ""]
                    #[doc = "# Note"]
                    #[doc = ""]
                    #[doc = "If the target is the depositor, the pool will be destroyed."]
                    withdraw_unbonded {
                        member_account: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        num_slashing_spans: ::core::primitive::u32,
                    },
                    #[codec(index = 6)]
                    #[doc = "Create a new delegation pool."]
                    #[doc = ""]
                    #[doc = "# Arguments"]
                    #[doc = ""]
                    #[doc = "* `amount` - The amount of funds to delegate to the pool. This also acts of a sort of"]
                    #[doc = "  deposit since the pools creator cannot fully unbond funds until the pool is being"]
                    #[doc = "  destroyed."]
                    #[doc = "* `index` - A disambiguation index for creating the account. Likely only useful when"]
                    #[doc = "  creating multiple pools in the same extrinsic."]
                    #[doc = "* `root` - The account to set as [`PoolRoles::root`]."]
                    #[doc = "* `nominator` - The account to set as the [`PoolRoles::nominator`]."]
                    #[doc = "* `state_toggler` - The account to set as the [`PoolRoles::state_toggler`]."]
                    #[doc = ""]
                    #[doc = "# Note"]
                    #[doc = ""]
                    #[doc = "In addition to `amount`, the caller will transfer the existential deposit; so the caller"]
                    #[doc = "needs at have at least `amount + existential_deposit` transferrable."]
                    create {
                        #[codec(compact)]
                        amount: ::core::primitive::u128,
                        root: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        nominator: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        state_toggler: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                    },
                    #[codec(index = 7)]
                    #[doc = "Create a new delegation pool with a previously used pool id"]
                    #[doc = ""]
                    #[doc = "# Arguments"]
                    #[doc = ""]
                    #[doc = "same as `create` with the inclusion of"]
                    #[doc = "* `pool_id` - `A valid PoolId."]
                    create_with_pool_id {
                        #[codec(compact)]
                        amount: ::core::primitive::u128,
                        root: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        nominator: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        state_toggler: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        pool_id: ::core::primitive::u32,
                    },
                    #[codec(index = 8)]
                    #[doc = "Nominate on behalf of the pool."]
                    #[doc = ""]
                    #[doc = "The dispatch origin of this call must be signed by the pool nominator or the pool"]
                    #[doc = "root role."]
                    #[doc = ""]
                    #[doc = "This directly forward the call to the staking pallet, on behalf of the pool bonded"]
                    #[doc = "account."]
                    nominate {
                        pool_id: ::core::primitive::u32,
                        validators: ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                    },
                    #[codec(index = 9)]
                    #[doc = "Set a new state for the pool."]
                    #[doc = ""]
                    #[doc = "If a pool is already in the `Destroying` state, then under no condition can its state"]
                    #[doc = "change again."]
                    #[doc = ""]
                    #[doc = "The dispatch origin of this call must be either:"]
                    #[doc = ""]
                    #[doc = "1. signed by the state toggler, or the root role of the pool,"]
                    #[doc = "2. if the pool conditions to be open are NOT met (as described by `ok_to_be_open`), and"]
                    #[doc = "   then the state of the pool can be permissionlessly changed to `Destroying`."]
                    set_state {
                        pool_id: ::core::primitive::u32,
                        state: runtime_types::pallet_nomination_pools::PoolState,
                    },
                    #[codec(index = 10)]
                    #[doc = "Set a new metadata for the pool."]
                    #[doc = ""]
                    #[doc = "The dispatch origin of this call must be signed by the state toggler, or the root role"]
                    #[doc = "of the pool."]
                    set_metadata {
                        pool_id: ::core::primitive::u32,
                        metadata: ::std::vec::Vec<::core::primitive::u8>,
                    },
                    #[codec(index = 11)]
                    #[doc = "Update configurations for the nomination pools. The origin for this call must be"]
                    #[doc = "Root."]
                    #[doc = ""]
                    #[doc = "# Arguments"]
                    #[doc = ""]
                    #[doc = "* `min_join_bond` - Set [`MinJoinBond`]."]
                    #[doc = "* `min_create_bond` - Set [`MinCreateBond`]."]
                    #[doc = "* `max_pools` - Set [`MaxPools`]."]
                    #[doc = "* `max_members` - Set [`MaxPoolMembers`]."]
                    #[doc = "* `max_members_per_pool` - Set [`MaxPoolMembersPerPool`]."]
                    set_configs {
                        min_join_bond: runtime_types::pallet_nomination_pools::ConfigOp<
                            ::core::primitive::u128,
                        >,
                        min_create_bond: runtime_types::pallet_nomination_pools::ConfigOp<
                            ::core::primitive::u128,
                        >,
                        max_pools: runtime_types::pallet_nomination_pools::ConfigOp<
                            ::core::primitive::u32,
                        >,
                        max_members: runtime_types::pallet_nomination_pools::ConfigOp<
                            ::core::primitive::u32,
                        >,
                        max_members_per_pool: runtime_types::pallet_nomination_pools::ConfigOp<
                            ::core::primitive::u32,
                        >,
                    },
                    #[codec(index = 12)]
                    #[doc = "Update the roles of the pool."]
                    #[doc = ""]
                    #[doc = "The root is the only entity that can change any of the roles, including itself,"]
                    #[doc = "excluding the depositor, who can never change."]
                    #[doc = ""]
                    #[doc = "It emits an event, notifying UIs of the role change. This event is quite relevant to"]
                    #[doc = "most pool members and they should be informed of changes to pool roles."]
                    update_roles {
                        pool_id: ::core::primitive::u32,
                        new_root: runtime_types::pallet_nomination_pools::ConfigOp<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                        >,
                        new_nominator: runtime_types::pallet_nomination_pools::ConfigOp<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                        >,
                        new_state_toggler: runtime_types::pallet_nomination_pools::ConfigOp<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                        >,
                    },
                    #[codec(index = 13)]
                    #[doc = "Chill on behalf of the pool."]
                    #[doc = ""]
                    #[doc = "The dispatch origin of this call must be signed by the pool nominator or the pool"]
                    #[doc = "root role, same as [`Pallet::nominate`]."]
                    #[doc = ""]
                    #[doc = "This directly forward the call to the staking pallet, on behalf of the pool bonded"]
                    #[doc = "account."]
                    chill { pool_id: ::core::primitive::u32 },
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub enum DefensiveError {
                    #[codec(index = 0)]
                    NotEnoughSpaceInUnbondPool,
                    #[codec(index = 1)]
                    PoolNotFound,
                    #[codec(index = 2)]
                    RewardPoolNotFound,
                    #[codec(index = 3)]
                    SubPoolsNotFound,
                    #[codec(index = 4)]
                    BondedStashKilledPrematurely,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tCustom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)\n\t\t\tof this pallet.\n\t\t\t"]
                pub enum Error {
                    #[codec(index = 0)]
                    #[doc = "A (bonded) pool id does not exist."]
                    PoolNotFound,
                    #[codec(index = 1)]
                    #[doc = "An account is not a member."]
                    PoolMemberNotFound,
                    #[codec(index = 2)]
                    #[doc = "A reward pool does not exist. In all cases this is a system logic error."]
                    RewardPoolNotFound,
                    #[codec(index = 3)]
                    #[doc = "A sub pool does not exist."]
                    SubPoolsNotFound,
                    #[codec(index = 4)]
                    #[doc = "An account is already delegating in another pool. An account may only belong to one"]
                    #[doc = "pool at a time."]
                    AccountBelongsToOtherPool,
                    #[codec(index = 5)]
                    #[doc = "The member is fully unbonded (and thus cannot access the bonded and reward pool"]
                    #[doc = "anymore to, for example, collect rewards)."]
                    FullyUnbonding,
                    #[codec(index = 6)]
                    #[doc = "The member cannot unbond further chunks due to reaching the limit."]
                    MaxUnbondingLimit,
                    #[codec(index = 7)]
                    #[doc = "None of the funds can be withdrawn yet because the bonding duration has not passed."]
                    CannotWithdrawAny,
                    #[codec(index = 8)]
                    #[doc = "The amount does not meet the minimum bond to either join or create a pool."]
                    #[doc = ""]
                    #[doc = "The depositor can never unbond to a value less than"]
                    #[doc = "`Pallet::depositor_min_bond`. The caller does not have nominating"]
                    #[doc = "permissions for the pool. Members can never unbond to a value below `MinJoinBond`."]
                    MinimumBondNotMet,
                    #[codec(index = 9)]
                    #[doc = "The transaction could not be executed due to overflow risk for the pool."]
                    OverflowRisk,
                    #[codec(index = 10)]
                    #[doc = "A pool must be in [`PoolState::Destroying`] in order for the depositor to unbond or for"]
                    #[doc = "other members to be permissionlessly unbonded."]
                    NotDestroying,
                    #[codec(index = 11)]
                    #[doc = "The caller does not have nominating permissions for the pool."]
                    NotNominator,
                    #[codec(index = 12)]
                    #[doc = "Either a) the caller cannot make a valid kick or b) the pool is not destroying."]
                    NotKickerOrDestroying,
                    #[codec(index = 13)]
                    #[doc = "The pool is not open to join"]
                    NotOpen,
                    #[codec(index = 14)]
                    #[doc = "The system is maxed out on pools."]
                    MaxPools,
                    #[codec(index = 15)]
                    #[doc = "Too many members in the pool or system."]
                    MaxPoolMembers,
                    #[codec(index = 16)]
                    #[doc = "The pools state cannot be changed."]
                    CanNotChangeState,
                    #[codec(index = 17)]
                    #[doc = "The caller does not have adequate permissions."]
                    DoesNotHavePermission,
                    #[codec(index = 18)]
                    #[doc = "Metadata exceeds [`Config::MaxMetadataLen`]"]
                    MetadataExceedsMaxLen,
                    #[codec(index = 19)]
                    #[doc = "Some error occurred that should never happen. This should be reported to the"]
                    #[doc = "maintainers."]
                    Defensive(runtime_types::pallet_nomination_pools::pallet::DefensiveError),
                    #[codec(index = 20)]
                    #[doc = "Partial unbonding now allowed permissionlessly."]
                    PartialUnbondNotAllowedPermissionlessly,
                    #[codec(index = 21)]
                    #[doc = "Pool id currently in use."]
                    PoolIdInUse,
                    #[codec(index = 22)]
                    #[doc = "Pool id provided is not correct/usable."]
                    InvalidPoolId,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Events of this pallet."]
                pub enum Event {
                    #[codec(index = 0)]
                    #[doc = "A pool has been created."]
                    Created {
                        depositor: ::subxt::ext::sp_core::crypto::AccountId32,
                        pool_id: ::core::primitive::u32,
                    },
                    #[codec(index = 1)]
                    #[doc = "A member has became bonded in a pool."]
                    Bonded {
                        member: ::subxt::ext::sp_core::crypto::AccountId32,
                        pool_id: ::core::primitive::u32,
                        bonded: ::core::primitive::u128,
                        joined: ::core::primitive::bool,
                    },
                    #[codec(index = 2)]
                    #[doc = "A payout has been made to a member."]
                    PaidOut {
                        member: ::subxt::ext::sp_core::crypto::AccountId32,
                        pool_id: ::core::primitive::u32,
                        payout: ::core::primitive::u128,
                    },
                    #[codec(index = 3)]
                    #[doc = "A member has unbonded from their pool."]
                    #[doc = ""]
                    #[doc = "- `balance` is the corresponding balance of the number of points that has been"]
                    #[doc = "  requested to be unbonded (the argument of the `unbond` transaction) from the bonded"]
                    #[doc = "  pool."]
                    #[doc = "- `points` is the number of points that are issued as a result of `balance` being"]
                    #[doc = "dissolved into the corresponding unbonding pool."]
                    #[doc = "- `era` is the era in which the balance will be unbonded."]
                    #[doc = "In the absence of slashing, these values will match. In the presence of slashing, the"]
                    #[doc = "number of points that are issued in the unbonding pool will be less than the amount"]
                    #[doc = "requested to be unbonded."]
                    Unbonded {
                        member: ::subxt::ext::sp_core::crypto::AccountId32,
                        pool_id: ::core::primitive::u32,
                        balance: ::core::primitive::u128,
                        points: ::core::primitive::u128,
                        era: ::core::primitive::u32,
                    },
                    #[codec(index = 4)]
                    #[doc = "A member has withdrawn from their pool."]
                    #[doc = ""]
                    #[doc = "The given number of `points` have been dissolved in return of `balance`."]
                    #[doc = ""]
                    #[doc = "Similar to `Unbonded` event, in the absence of slashing, the ratio of point to balance"]
                    #[doc = "will be 1."]
                    Withdrawn {
                        member: ::subxt::ext::sp_core::crypto::AccountId32,
                        pool_id: ::core::primitive::u32,
                        balance: ::core::primitive::u128,
                        points: ::core::primitive::u128,
                    },
                    #[codec(index = 5)]
                    #[doc = "A pool has been destroyed."]
                    Destroyed { pool_id: ::core::primitive::u32 },
                    #[codec(index = 6)]
                    #[doc = "The state of a pool has changed"]
                    StateChanged {
                        pool_id: ::core::primitive::u32,
                        new_state: runtime_types::pallet_nomination_pools::PoolState,
                    },
                    #[codec(index = 7)]
                    #[doc = "A member has been removed from a pool."]
                    #[doc = ""]
                    #[doc = "The removal can be voluntary (withdrawn all unbonded funds) or involuntary (kicked)."]
                    MemberRemoved {
                        pool_id: ::core::primitive::u32,
                        member: ::subxt::ext::sp_core::crypto::AccountId32,
                    },
                    #[codec(index = 8)]
                    #[doc = "The roles of a pool have been updated to the given new roles. Note that the depositor"]
                    #[doc = "can never change."]
                    RolesUpdated {
                        root: ::core::option::Option<::subxt::ext::sp_core::crypto::AccountId32>,
                        state_toggler:
                            ::core::option::Option<::subxt::ext::sp_core::crypto::AccountId32>,
                        nominator:
                            ::core::option::Option<::subxt::ext::sp_core::crypto::AccountId32>,
                    },
                    #[codec(index = 9)]
                    #[doc = "The active balance of pool `pool_id` has been slashed to `balance`."]
                    PoolSlashed {
                        pool_id: ::core::primitive::u32,
                        balance: ::core::primitive::u128,
                    },
                    #[codec(index = 10)]
                    #[doc = "The unbond pool at `era` of pool `pool_id` has been slashed to `balance`."]
                    UnbondingPoolSlashed {
                        pool_id: ::core::primitive::u32,
                        era: ::core::primitive::u32,
                        balance: ::core::primitive::u128,
                    },
                }
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum BondExtra<_0> {
                #[codec(index = 0)]
                FreeBalance(_0),
                #[codec(index = 1)]
                Rewards,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct BondedPoolInner {
                pub points: ::core::primitive::u128,
                pub state: runtime_types::pallet_nomination_pools::PoolState,
                pub member_counter: ::core::primitive::u32,
                pub roles: runtime_types::pallet_nomination_pools::PoolRoles<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum ConfigOp<_0> {
                #[codec(index = 0)]
                Noop,
                #[codec(index = 1)]
                Set(_0),
                #[codec(index = 2)]
                Remove,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct PoolMember {
                pub pool_id: ::core::primitive::u32,
                pub points: ::core::primitive::u128,
                pub last_recorded_reward_counter:
                    runtime_types::sp_arithmetic::fixed_point::FixedU128,
                pub unbonding_eras:
                    runtime_types::sp_core::bounded::bounded_btree_map::BoundedBTreeMap<
                        ::core::primitive::u32,
                        ::core::primitive::u128,
                    >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct PoolRoles<_0> {
                pub depositor: _0,
                pub root: ::core::option::Option<_0>,
                pub nominator: ::core::option::Option<_0>,
                pub state_toggler: ::core::option::Option<_0>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum PoolState {
                #[codec(index = 0)]
                Open,
                #[codec(index = 1)]
                Blocked,
                #[codec(index = 2)]
                Destroying,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct RewardPool {
                pub last_recorded_reward_counter:
                    runtime_types::sp_arithmetic::fixed_point::FixedU128,
                pub last_recorded_total_payouts: ::core::primitive::u128,
                pub total_rewards_claimed: ::core::primitive::u128,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct SubPools {
                pub no_era: runtime_types::pallet_nomination_pools::UnbondPool,
                pub with_era: runtime_types::sp_core::bounded::bounded_btree_map::BoundedBTreeMap<
                    ::core::primitive::u32,
                    runtime_types::pallet_nomination_pools::UnbondPool,
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct UnbondPool {
                pub points: ::core::primitive::u128,
                pub balance: ::core::primitive::u128,
            }
        }
        pub mod pallet_scheduler {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
                pub enum Call {
                    #[codec(index = 0)]
                    #[doc = "Anonymously schedule a task."]
                    schedule {
                        when: ::core::primitive::u32,
                        maybe_periodic: ::core::option::Option<(
                            ::core::primitive::u32,
                            ::core::primitive::u32,
                        )>,
                        priority: ::core::primitive::u8,
                        call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
                    },
                    #[codec(index = 1)]
                    #[doc = "Cancel an anonymously scheduled task."]
                    cancel {
                        when: ::core::primitive::u32,
                        index: ::core::primitive::u32,
                    },
                    #[codec(index = 2)]
                    #[doc = "Schedule a named task."]
                    schedule_named {
                        id: [::core::primitive::u8; 32usize],
                        when: ::core::primitive::u32,
                        maybe_periodic: ::core::option::Option<(
                            ::core::primitive::u32,
                            ::core::primitive::u32,
                        )>,
                        priority: ::core::primitive::u8,
                        call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
                    },
                    #[codec(index = 3)]
                    #[doc = "Cancel a named scheduled task."]
                    cancel_named {
                        id: [::core::primitive::u8; 32usize],
                    },
                    #[codec(index = 4)]
                    #[doc = "Anonymously schedule a task after a delay."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "Same as [`schedule`]."]
                    #[doc = "# </weight>"]
                    schedule_after {
                        after: ::core::primitive::u32,
                        maybe_periodic: ::core::option::Option<(
                            ::core::primitive::u32,
                            ::core::primitive::u32,
                        )>,
                        priority: ::core::primitive::u8,
                        call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
                    },
                    #[codec(index = 5)]
                    #[doc = "Schedule a named task after a delay."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "Same as [`schedule_named`](Self::schedule_named)."]
                    #[doc = "# </weight>"]
                    schedule_named_after {
                        id: [::core::primitive::u8; 32usize],
                        after: ::core::primitive::u32,
                        maybe_periodic: ::core::option::Option<(
                            ::core::primitive::u32,
                            ::core::primitive::u32,
                        )>,
                        priority: ::core::primitive::u8,
                        call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
                    },
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tCustom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)\n\t\t\tof this pallet.\n\t\t\t"]
                pub enum Error {
                    #[codec(index = 0)]
                    #[doc = "Failed to schedule a call"]
                    FailedToSchedule,
                    #[codec(index = 1)]
                    #[doc = "Cannot find the scheduled call."]
                    NotFound,
                    #[codec(index = 2)]
                    #[doc = "Given target block number is in the past."]
                    TargetBlockNumberInPast,
                    #[codec(index = 3)]
                    #[doc = "Reschedule failed because it does not change scheduled time."]
                    RescheduleNoChange,
                    #[codec(index = 4)]
                    #[doc = "Attempt to use a non-named function on a named task."]
                    Named,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Events type."]
                pub enum Event {
                    #[codec(index = 0)]
                    #[doc = "Scheduled some task."]
                    Scheduled {
                        when: ::core::primitive::u32,
                        index: ::core::primitive::u32,
                    },
                    #[codec(index = 1)]
                    #[doc = "Canceled some task."]
                    Canceled {
                        when: ::core::primitive::u32,
                        index: ::core::primitive::u32,
                    },
                    #[codec(index = 2)]
                    #[doc = "Dispatched some task."]
                    Dispatched {
                        task: (::core::primitive::u32, ::core::primitive::u32),
                        id: ::core::option::Option<[::core::primitive::u8; 32usize]>,
                        result:
                            ::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
                    },
                    #[codec(index = 3)]
                    #[doc = "The call for the provided hash was not found so the task has been aborted."]
                    CallUnavailable {
                        task: (::core::primitive::u32, ::core::primitive::u32),
                        id: ::core::option::Option<[::core::primitive::u8; 32usize]>,
                    },
                    #[codec(index = 4)]
                    #[doc = "The given task was unable to be renewed since the agenda is full at that block."]
                    PeriodicFailed {
                        task: (::core::primitive::u32, ::core::primitive::u32),
                        id: ::core::option::Option<[::core::primitive::u8; 32usize]>,
                    },
                    #[codec(index = 5)]
                    #[doc = "The given task can never be executed since it is overweight."]
                    PermanentlyOverweight {
                        task: (::core::primitive::u32, ::core::primitive::u32),
                        id: ::core::option::Option<[::core::primitive::u8; 32usize]>,
                    },
                }
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Scheduled<_0, _1, _2, _3, _4> {
                pub maybe_id: ::core::option::Option<_0>,
                pub priority: ::core::primitive::u8,
                pub call: _1,
                pub maybe_periodic: ::core::option::Option<(_2, _2)>,
                pub origin: _3,
                #[codec(skip)]
                pub __subxt_unused_type_params: ::core::marker::PhantomData<_4>,
            }
        }
        pub mod pallet_session {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
                pub enum Call {
                    #[codec(index = 0)]
                    #[doc = "Sets the session key(s) of the function caller to `keys`."]
                    #[doc = "Allows an account to set its session key prior to becoming a validator."]
                    #[doc = "This doesn't take effect until the next session."]
                    #[doc = ""]
                    #[doc = "The dispatch origin of this function must be signed."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- Complexity: `O(1)`. Actual cost depends on the number of length of"]
                    #[doc = "  `T::Keys::key_ids()` which is fixed."]
                    #[doc = "- DbReads: `origin account`, `T::ValidatorIdOf`, `NextKeys`"]
                    #[doc = "- DbWrites: `origin account`, `NextKeys`"]
                    #[doc = "- DbReads per key id: `KeyOwner`"]
                    #[doc = "- DbWrites per key id: `KeyOwner`"]
                    #[doc = "# </weight>"]
                    set_keys {
                        keys: runtime_types::aleph_runtime::SessionKeys,
                        proof: ::std::vec::Vec<::core::primitive::u8>,
                    },
                    #[codec(index = 1)]
                    #[doc = "Removes any session key(s) of the function caller."]
                    #[doc = ""]
                    #[doc = "This doesn't take effect until the next session."]
                    #[doc = ""]
                    #[doc = "The dispatch origin of this function must be Signed and the account must be either be"]
                    #[doc = "convertible to a validator ID using the chain's typical addressing system (this usually"]
                    #[doc = "means being a controller account) or directly convertible into a validator ID (which"]
                    #[doc = "usually means being a stash account)."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- Complexity: `O(1)` in number of key types. Actual cost depends on the number of length"]
                    #[doc = "  of `T::Keys::key_ids()` which is fixed."]
                    #[doc = "- DbReads: `T::ValidatorIdOf`, `NextKeys`, `origin account`"]
                    #[doc = "- DbWrites: `NextKeys`, `origin account`"]
                    #[doc = "- DbWrites per key id: `KeyOwner`"]
                    #[doc = "# </weight>"]
                    purge_keys,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Error for the session pallet."]
                pub enum Error {
                    #[codec(index = 0)]
                    #[doc = "Invalid ownership proof."]
                    InvalidProof,
                    #[codec(index = 1)]
                    #[doc = "No associated validator ID for account."]
                    NoAssociatedValidatorId,
                    #[codec(index = 2)]
                    #[doc = "Registered duplicate key."]
                    DuplicatedKey,
                    #[codec(index = 3)]
                    #[doc = "No keys are associated with this account."]
                    NoKeys,
                    #[codec(index = 4)]
                    #[doc = "Key setting account is not live, so it's impossible to associate keys."]
                    NoAccount,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
                pub enum Event {
                    #[codec(index = 0)]
                    #[doc = "New session has happened. Note that the argument is the session index, not the"]
                    #[doc = "block number as the type might suggest."]
                    NewSession {
                        session_index: ::core::primitive::u32,
                    },
                }
            }
        }
        pub mod pallet_staking {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                pub mod pallet {
                    use super::runtime_types;
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
                    pub enum Call {
                        #[codec(index = 0)]
                        #[doc = "Take the origin account as a stash and lock up `value` of its balance. `controller` will"]
                        #[doc = "be the account that controls it."]
                        #[doc = ""]
                        #[doc = "`value` must be more than the `minimum_balance` specified by `T::Currency`."]
                        #[doc = ""]
                        #[doc = "The dispatch origin for this call must be _Signed_ by the stash account."]
                        #[doc = ""]
                        #[doc = "Emits `Bonded`."]
                        #[doc = "# <weight>"]
                        #[doc = "- Independent of the arguments. Moderate complexity."]
                        #[doc = "- O(1)."]
                        #[doc = "- Three extra DB entries."]
                        #[doc = ""]
                        #[doc = "NOTE: Two of the storage writes (`Self::bonded`, `Self::payee`) are _never_ cleaned"]
                        #[doc = "unless the `origin` falls below _existential deposit_ and gets removed as dust."]
                        #[doc = "------------------"]
                        #[doc = "# </weight>"]
                        bond {
                            controller: ::subxt::ext::sp_runtime::MultiAddress<
                                ::subxt::ext::sp_core::crypto::AccountId32,
                                (),
                            >,
                            #[codec(compact)]
                            value: ::core::primitive::u128,
                            payee: runtime_types::pallet_staking::RewardDestination<
                                ::subxt::ext::sp_core::crypto::AccountId32,
                            >,
                        },
                        #[codec(index = 1)]
                        #[doc = "Add some extra amount that have appeared in the stash `free_balance` into the balance up"]
                        #[doc = "for staking."]
                        #[doc = ""]
                        #[doc = "The dispatch origin for this call must be _Signed_ by the stash, not the controller."]
                        #[doc = ""]
                        #[doc = "Use this if there are additional funds in your stash account that you wish to bond."]
                        #[doc = "Unlike [`bond`](Self::bond) or [`unbond`](Self::unbond) this function does not impose"]
                        #[doc = "any limitation on the amount that can be added."]
                        #[doc = ""]
                        #[doc = "Emits `Bonded`."]
                        #[doc = ""]
                        #[doc = "# <weight>"]
                        #[doc = "- Independent of the arguments. Insignificant complexity."]
                        #[doc = "- O(1)."]
                        #[doc = "# </weight>"]
                        bond_extra {
                            #[codec(compact)]
                            max_additional: ::core::primitive::u128,
                        },
                        #[codec(index = 2)]
                        #[doc = "Schedule a portion of the stash to be unlocked ready for transfer out after the bond"]
                        #[doc = "period ends. If this leaves an amount actively bonded less than"]
                        #[doc = "T::Currency::minimum_balance(), then it is increased to the full amount."]
                        #[doc = ""]
                        #[doc = "The dispatch origin for this call must be _Signed_ by the controller, not the stash."]
                        #[doc = ""]
                        #[doc = "Once the unlock period is done, you can call `withdraw_unbonded` to actually move"]
                        #[doc = "the funds out of management ready for transfer."]
                        #[doc = ""]
                        #[doc = "No more than a limited number of unlocking chunks (see `MaxUnlockingChunks`)"]
                        #[doc = "can co-exists at the same time. If there are no unlocking chunks slots available"]
                        #[doc = "[`Call::withdraw_unbonded`] is called to remove some of the chunks (if possible)."]
                        #[doc = ""]
                        #[doc = "If a user encounters the `InsufficientBond` error when calling this extrinsic,"]
                        #[doc = "they should call `chill` first in order to free up their bonded funds."]
                        #[doc = ""]
                        #[doc = "Emits `Unbonded`."]
                        #[doc = ""]
                        #[doc = "See also [`Call::withdraw_unbonded`]."]
                        unbond {
                            #[codec(compact)]
                            value: ::core::primitive::u128,
                        },
                        #[codec(index = 3)]
                        #[doc = "Remove any unlocked chunks from the `unlocking` queue from our management."]
                        #[doc = ""]
                        #[doc = "This essentially frees up that balance to be used by the stash account to do"]
                        #[doc = "whatever it wants."]
                        #[doc = ""]
                        #[doc = "The dispatch origin for this call must be _Signed_ by the controller."]
                        #[doc = ""]
                        #[doc = "Emits `Withdrawn`."]
                        #[doc = ""]
                        #[doc = "See also [`Call::unbond`]."]
                        #[doc = ""]
                        #[doc = "# <weight>"]
                        #[doc = "Complexity O(S) where S is the number of slashing spans to remove"]
                        #[doc = "NOTE: Weight annotation is the kill scenario, we refund otherwise."]
                        #[doc = "# </weight>"]
                        withdraw_unbonded {
                            num_slashing_spans: ::core::primitive::u32,
                        },
                        #[codec(index = 4)]
                        #[doc = "Declare the desire to validate for the origin controller."]
                        #[doc = ""]
                        #[doc = "Effects will be felt at the beginning of the next era."]
                        #[doc = ""]
                        #[doc = "The dispatch origin for this call must be _Signed_ by the controller, not the stash."]
                        validate {
                            prefs: runtime_types::pallet_staking::ValidatorPrefs,
                        },
                        #[codec(index = 5)]
                        #[doc = "Declare the desire to nominate `targets` for the origin controller."]
                        #[doc = ""]
                        #[doc = "Effects will be felt at the beginning of the next era."]
                        #[doc = ""]
                        #[doc = "The dispatch origin for this call must be _Signed_ by the controller, not the stash."]
                        #[doc = ""]
                        #[doc = "# <weight>"]
                        #[doc = "- The transaction's complexity is proportional to the size of `targets` (N)"]
                        #[doc = "which is capped at CompactAssignments::LIMIT (T::MaxNominations)."]
                        #[doc = "- Both the reads and writes follow a similar pattern."]
                        #[doc = "# </weight>"]
                        nominate {
                            targets: ::std::vec::Vec<
                                ::subxt::ext::sp_runtime::MultiAddress<
                                    ::subxt::ext::sp_core::crypto::AccountId32,
                                    (),
                                >,
                            >,
                        },
                        #[codec(index = 6)]
                        #[doc = "Declare no desire to either validate or nominate."]
                        #[doc = ""]
                        #[doc = "Effects will be felt at the beginning of the next era."]
                        #[doc = ""]
                        #[doc = "The dispatch origin for this call must be _Signed_ by the controller, not the stash."]
                        #[doc = ""]
                        #[doc = "# <weight>"]
                        #[doc = "- Independent of the arguments. Insignificant complexity."]
                        #[doc = "- Contains one read."]
                        #[doc = "- Writes are limited to the `origin` account key."]
                        #[doc = "# </weight>"]
                        chill,
                        #[codec(index = 7)]
                        #[doc = "(Re-)set the payment target for a controller."]
                        #[doc = ""]
                        #[doc = "Effects will be felt instantly (as soon as this function is completed successfully)."]
                        #[doc = ""]
                        #[doc = "The dispatch origin for this call must be _Signed_ by the controller, not the stash."]
                        #[doc = ""]
                        #[doc = "# <weight>"]
                        #[doc = "- Independent of the arguments. Insignificant complexity."]
                        #[doc = "- Contains a limited number of reads."]
                        #[doc = "- Writes are limited to the `origin` account key."]
                        #[doc = "---------"]
                        #[doc = "- Weight: O(1)"]
                        #[doc = "- DB Weight:"]
                        #[doc = "    - Read: Ledger"]
                        #[doc = "    - Write: Payee"]
                        #[doc = "# </weight>"]
                        set_payee {
                            payee: runtime_types::pallet_staking::RewardDestination<
                                ::subxt::ext::sp_core::crypto::AccountId32,
                            >,
                        },
                        #[codec(index = 8)]
                        #[doc = "(Re-)set the controller of a stash."]
                        #[doc = ""]
                        #[doc = "Effects will be felt instantly (as soon as this function is completed successfully)."]
                        #[doc = ""]
                        #[doc = "The dispatch origin for this call must be _Signed_ by the stash, not the controller."]
                        #[doc = ""]
                        #[doc = "# <weight>"]
                        #[doc = "- Independent of the arguments. Insignificant complexity."]
                        #[doc = "- Contains a limited number of reads."]
                        #[doc = "- Writes are limited to the `origin` account key."]
                        #[doc = "----------"]
                        #[doc = "Weight: O(1)"]
                        #[doc = "DB Weight:"]
                        #[doc = "- Read: Bonded, Ledger New Controller, Ledger Old Controller"]
                        #[doc = "- Write: Bonded, Ledger New Controller, Ledger Old Controller"]
                        #[doc = "# </weight>"]
                        set_controller {
                            controller: ::subxt::ext::sp_runtime::MultiAddress<
                                ::subxt::ext::sp_core::crypto::AccountId32,
                                (),
                            >,
                        },
                        #[codec(index = 9)]
                        #[doc = "Sets the ideal number of validators."]
                        #[doc = ""]
                        #[doc = "The dispatch origin must be Root."]
                        #[doc = ""]
                        #[doc = "# <weight>"]
                        #[doc = "Weight: O(1)"]
                        #[doc = "Write: Validator Count"]
                        #[doc = "# </weight>"]
                        set_validator_count {
                            #[codec(compact)]
                            new: ::core::primitive::u32,
                        },
                        #[codec(index = 10)]
                        #[doc = "Increments the ideal number of validators upto maximum of"]
                        #[doc = "`ElectionProviderBase::MaxWinners`."]
                        #[doc = ""]
                        #[doc = "The dispatch origin must be Root."]
                        #[doc = ""]
                        #[doc = "# <weight>"]
                        #[doc = "Same as [`Self::set_validator_count`]."]
                        #[doc = "# </weight>"]
                        increase_validator_count {
                            #[codec(compact)]
                            additional: ::core::primitive::u32,
                        },
                        #[codec(index = 11)]
                        #[doc = "Scale up the ideal number of validators by a factor upto maximum of"]
                        #[doc = "`ElectionProviderBase::MaxWinners`."]
                        #[doc = ""]
                        #[doc = "The dispatch origin must be Root."]
                        #[doc = ""]
                        #[doc = "# <weight>"]
                        #[doc = "Same as [`Self::set_validator_count`]."]
                        #[doc = "# </weight>"]
                        scale_validator_count {
                            factor: runtime_types::sp_arithmetic::per_things::Percent,
                        },
                        #[codec(index = 12)]
                        #[doc = "Force there to be no new eras indefinitely."]
                        #[doc = ""]
                        #[doc = "The dispatch origin must be Root."]
                        #[doc = ""]
                        #[doc = "# Warning"]
                        #[doc = ""]
                        #[doc = "The election process starts multiple blocks before the end of the era."]
                        #[doc = "Thus the election process may be ongoing when this is called. In this case the"]
                        #[doc = "election will continue until the next era is triggered."]
                        #[doc = ""]
                        #[doc = "# <weight>"]
                        #[doc = "- No arguments."]
                        #[doc = "- Weight: O(1)"]
                        #[doc = "- Write: ForceEra"]
                        #[doc = "# </weight>"]
                        force_no_eras,
                        #[codec(index = 13)]
                        #[doc = "Force there to be a new era at the end of the next session. After this, it will be"]
                        #[doc = "reset to normal (non-forced) behaviour."]
                        #[doc = ""]
                        #[doc = "The dispatch origin must be Root."]
                        #[doc = ""]
                        #[doc = "# Warning"]
                        #[doc = ""]
                        #[doc = "The election process starts multiple blocks before the end of the era."]
                        #[doc = "If this is called just before a new era is triggered, the election process may not"]
                        #[doc = "have enough blocks to get a result."]
                        #[doc = ""]
                        #[doc = "# <weight>"]
                        #[doc = "- No arguments."]
                        #[doc = "- Weight: O(1)"]
                        #[doc = "- Write ForceEra"]
                        #[doc = "# </weight>"]
                        force_new_era,
                        #[codec(index = 14)]
                        #[doc = "Set the validators who cannot be slashed (if any)."]
                        #[doc = ""]
                        #[doc = "The dispatch origin must be Root."]
                        set_invulnerables {
                            invulnerables:
                                ::std::vec::Vec<::subxt::ext::sp_core::crypto::AccountId32>,
                        },
                        #[codec(index = 15)]
                        #[doc = "Force a current staker to become completely unstaked, immediately."]
                        #[doc = ""]
                        #[doc = "The dispatch origin must be Root."]
                        force_unstake {
                            stash: ::subxt::ext::sp_core::crypto::AccountId32,
                            num_slashing_spans: ::core::primitive::u32,
                        },
                        #[codec(index = 16)]
                        #[doc = "Force there to be a new era at the end of sessions indefinitely."]
                        #[doc = ""]
                        #[doc = "The dispatch origin must be Root."]
                        #[doc = ""]
                        #[doc = "# Warning"]
                        #[doc = ""]
                        #[doc = "The election process starts multiple blocks before the end of the era."]
                        #[doc = "If this is called just before a new era is triggered, the election process may not"]
                        #[doc = "have enough blocks to get a result."]
                        force_new_era_always,
                        #[codec(index = 17)]
                        #[doc = "Cancel enactment of a deferred slash."]
                        #[doc = ""]
                        #[doc = "Can be called by the `T::AdminOrigin`."]
                        #[doc = ""]
                        #[doc = "Parameters: era and indices of the slashes for that era to kill."]
                        cancel_deferred_slash {
                            era: ::core::primitive::u32,
                            slash_indices: ::std::vec::Vec<::core::primitive::u32>,
                        },
                        #[codec(index = 18)]
                        #[doc = "Pay out all the stakers behind a single validator for a single era."]
                        #[doc = ""]
                        #[doc = "- `validator_stash` is the stash account of the validator. Their nominators, up to"]
                        #[doc = "  `T::MaxNominatorRewardedPerValidator`, will also receive their rewards."]
                        #[doc = "- `era` may be any era between `[current_era - history_depth; current_era]`."]
                        #[doc = ""]
                        #[doc = "The origin of this call must be _Signed_. Any account can call this function, even if"]
                        #[doc = "it is not one of the stakers."]
                        #[doc = ""]
                        #[doc = "# <weight>"]
                        #[doc = "- Time complexity: at most O(MaxNominatorRewardedPerValidator)."]
                        #[doc = "- Contains a limited number of reads and writes."]
                        #[doc = "-----------"]
                        #[doc = "N is the Number of payouts for the validator (including the validator)"]
                        #[doc = "Weight:"]
                        #[doc = "- Reward Destination Staked: O(N)"]
                        #[doc = "- Reward Destination Controller (Creating): O(N)"]
                        #[doc = ""]
                        #[doc = "  NOTE: weights are assuming that payouts are made to alive stash account (Staked)."]
                        #[doc = "  Paying even a dead controller is cheaper weight-wise. We don't do any refunds here."]
                        #[doc = "# </weight>"]
                        payout_stakers {
                            validator_stash: ::subxt::ext::sp_core::crypto::AccountId32,
                            era: ::core::primitive::u32,
                        },
                        #[codec(index = 19)]
                        #[doc = "Rebond a portion of the stash scheduled to be unlocked."]
                        #[doc = ""]
                        #[doc = "The dispatch origin must be signed by the controller."]
                        #[doc = ""]
                        #[doc = "# <weight>"]
                        #[doc = "- Time complexity: O(L), where L is unlocking chunks"]
                        #[doc = "- Bounded by `MaxUnlockingChunks`."]
                        #[doc = "- Storage changes: Can't increase storage, only decrease it."]
                        #[doc = "# </weight>"]
                        rebond {
                            #[codec(compact)]
                            value: ::core::primitive::u128,
                        },
                        #[codec(index = 20)]
                        #[doc = "Remove all data structures concerning a staker/stash once it is at a state where it can"]
                        #[doc = "be considered `dust` in the staking system. The requirements are:"]
                        #[doc = ""]
                        #[doc = "1. the `total_balance` of the stash is below existential deposit."]
                        #[doc = "2. or, the `ledger.total` of the stash is below existential deposit."]
                        #[doc = ""]
                        #[doc = "The former can happen in cases like a slash; the latter when a fully unbonded account"]
                        #[doc = "is still receiving staking rewards in `RewardDestination::Staked`."]
                        #[doc = ""]
                        #[doc = "It can be called by anyone, as long as `stash` meets the above requirements."]
                        #[doc = ""]
                        #[doc = "Refunds the transaction fees upon successful execution."]
                        reap_stash {
                            stash: ::subxt::ext::sp_core::crypto::AccountId32,
                            num_slashing_spans: ::core::primitive::u32,
                        },
                        #[codec(index = 21)]
                        #[doc = "Remove the given nominations from the calling validator."]
                        #[doc = ""]
                        #[doc = "Effects will be felt at the beginning of the next era."]
                        #[doc = ""]
                        #[doc = "The dispatch origin for this call must be _Signed_ by the controller, not the stash."]
                        #[doc = ""]
                        #[doc = "- `who`: A list of nominator stash accounts who are nominating this validator which"]
                        #[doc = "  should no longer be nominating this validator."]
                        #[doc = ""]
                        #[doc = "Note: Making this call only makes sense if you first set the validator preferences to"]
                        #[doc = "block any further nominations."]
                        kick {
                            who: ::std::vec::Vec<
                                ::subxt::ext::sp_runtime::MultiAddress<
                                    ::subxt::ext::sp_core::crypto::AccountId32,
                                    (),
                                >,
                            >,
                        },
                        #[codec(index = 22)]
                        #[doc = "Update the various staking configurations ."]
                        #[doc = ""]
                        #[doc = "* `min_nominator_bond`: The minimum active bond needed to be a nominator."]
                        #[doc = "* `min_validator_bond`: The minimum active bond needed to be a validator."]
                        #[doc = "* `max_nominator_count`: The max number of users who can be a nominator at once. When"]
                        #[doc = "  set to `None`, no limit is enforced."]
                        #[doc = "* `max_validator_count`: The max number of users who can be a validator at once. When"]
                        #[doc = "  set to `None`, no limit is enforced."]
                        #[doc = "* `chill_threshold`: The ratio of `max_nominator_count` or `max_validator_count` which"]
                        #[doc = "  should be filled in order for the `chill_other` transaction to work."]
                        #[doc = "* `min_commission`: The minimum amount of commission that each validators must maintain."]
                        #[doc = "  This is checked only upon calling `validate`. Existing validators are not affected."]
                        #[doc = ""]
                        #[doc = "RuntimeOrigin must be Root to call this function."]
                        #[doc = ""]
                        #[doc = "NOTE: Existing nominators and validators will not be affected by this update."]
                        #[doc = "to kick people under the new limits, `chill_other` should be called."]
                        set_staking_configs {
                            min_nominator_bond:
                                runtime_types::pallet_staking::pallet::pallet::ConfigOp<
                                    ::core::primitive::u128,
                                >,
                            min_validator_bond:
                                runtime_types::pallet_staking::pallet::pallet::ConfigOp<
                                    ::core::primitive::u128,
                                >,
                            max_nominator_count:
                                runtime_types::pallet_staking::pallet::pallet::ConfigOp<
                                    ::core::primitive::u32,
                                >,
                            max_validator_count:
                                runtime_types::pallet_staking::pallet::pallet::ConfigOp<
                                    ::core::primitive::u32,
                                >,
                            chill_threshold:
                                runtime_types::pallet_staking::pallet::pallet::ConfigOp<
                                    runtime_types::sp_arithmetic::per_things::Percent,
                                >,
                            min_commission: runtime_types::pallet_staking::pallet::pallet::ConfigOp<
                                runtime_types::sp_arithmetic::per_things::Perbill,
                            >,
                        },
                        #[codec(index = 23)]
                        #[doc = "Declare a `controller` to stop participating as either a validator or nominator."]
                        #[doc = ""]
                        #[doc = "Effects will be felt at the beginning of the next era."]
                        #[doc = ""]
                        #[doc = "The dispatch origin for this call must be _Signed_, but can be called by anyone."]
                        #[doc = ""]
                        #[doc = "If the caller is the same as the controller being targeted, then no further checks are"]
                        #[doc = "enforced, and this function behaves just like `chill`."]
                        #[doc = ""]
                        #[doc = "If the caller is different than the controller being targeted, the following conditions"]
                        #[doc = "must be met:"]
                        #[doc = ""]
                        #[doc = "* `controller` must belong to a nominator who has become non-decodable,"]
                        #[doc = ""]
                        #[doc = "Or:"]
                        #[doc = ""]
                        #[doc = "* A `ChillThreshold` must be set and checked which defines how close to the max"]
                        #[doc = "  nominators or validators we must reach before users can start chilling one-another."]
                        #[doc = "* A `MaxNominatorCount` and `MaxValidatorCount` must be set which is used to determine"]
                        #[doc = "  how close we are to the threshold."]
                        #[doc = "* A `MinNominatorBond` and `MinValidatorBond` must be set and checked, which determines"]
                        #[doc = "  if this is a person that should be chilled because they have not met the threshold"]
                        #[doc = "  bond required."]
                        #[doc = ""]
                        #[doc = "This can be helpful if bond requirements are updated, and we need to remove old users"]
                        #[doc = "who do not satisfy these requirements."]
                        chill_other {
                            controller: ::subxt::ext::sp_core::crypto::AccountId32,
                        },
                        #[codec(index = 24)]
                        #[doc = "Force a validator to have at least the minimum commission. This will not affect a"]
                        #[doc = "validator who already has a commission greater than or equal to the minimum. Any account"]
                        #[doc = "can call this."]
                        force_apply_min_commission {
                            validator_stash: ::subxt::ext::sp_core::crypto::AccountId32,
                        },
                        #[codec(index = 25)]
                        #[doc = "Sets the minimum amount of commission that each validators must maintain."]
                        #[doc = ""]
                        #[doc = "This call has lower privilege requirements than `set_staking_config` and can be called"]
                        #[doc = "by the `T::AdminOrigin`. Root can always call this."]
                        set_min_commission {
                            new: runtime_types::sp_arithmetic::per_things::Perbill,
                        },
                    }
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    pub enum ConfigOp<_0> {
                        #[codec(index = 0)]
                        Noop,
                        #[codec(index = 1)]
                        Set(_0),
                        #[codec(index = 2)]
                        Remove,
                    }
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    #[doc = "\n\t\t\tCustom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)\n\t\t\tof this pallet.\n\t\t\t"]
                    pub enum Error {
                        #[codec(index = 0)]
                        #[doc = "Not a controller account."]
                        NotController,
                        #[codec(index = 1)]
                        #[doc = "Not a stash account."]
                        NotStash,
                        #[codec(index = 2)]
                        #[doc = "Stash is already bonded."]
                        AlreadyBonded,
                        #[codec(index = 3)]
                        #[doc = "Controller is already paired."]
                        AlreadyPaired,
                        #[codec(index = 4)]
                        #[doc = "Targets cannot be empty."]
                        EmptyTargets,
                        #[codec(index = 5)]
                        #[doc = "Duplicate index."]
                        DuplicateIndex,
                        #[codec(index = 6)]
                        #[doc = "Slash record index out of bounds."]
                        InvalidSlashIndex,
                        #[codec(index = 7)]
                        #[doc = "Cannot have a validator or nominator role, with value less than the minimum defined by"]
                        #[doc = "governance (see `MinValidatorBond` and `MinNominatorBond`). If unbonding is the"]
                        #[doc = "intention, `chill` first to remove one's role as validator/nominator."]
                        InsufficientBond,
                        #[codec(index = 8)]
                        #[doc = "Can not schedule more unlock chunks."]
                        NoMoreChunks,
                        #[codec(index = 9)]
                        #[doc = "Can not rebond without unlocking chunks."]
                        NoUnlockChunk,
                        #[codec(index = 10)]
                        #[doc = "Attempting to target a stash that still has funds."]
                        FundedTarget,
                        #[codec(index = 11)]
                        #[doc = "Invalid era to reward."]
                        InvalidEraToReward,
                        #[codec(index = 12)]
                        #[doc = "Invalid number of nominations."]
                        InvalidNumberOfNominations,
                        #[codec(index = 13)]
                        #[doc = "Items are not sorted and unique."]
                        NotSortedAndUnique,
                        #[codec(index = 14)]
                        #[doc = "Rewards for this era have already been claimed for this validator."]
                        AlreadyClaimed,
                        #[codec(index = 15)]
                        #[doc = "Incorrect previous history depth input provided."]
                        IncorrectHistoryDepth,
                        #[codec(index = 16)]
                        #[doc = "Incorrect number of slashing spans provided."]
                        IncorrectSlashingSpans,
                        #[codec(index = 17)]
                        #[doc = "Internal state has become somehow corrupted and the operation cannot continue."]
                        BadState,
                        #[codec(index = 18)]
                        #[doc = "Too many nomination targets supplied."]
                        TooManyTargets,
                        #[codec(index = 19)]
                        #[doc = "A nomination target was supplied that was blocked or otherwise not a validator."]
                        BadTarget,
                        #[codec(index = 20)]
                        #[doc = "The user has enough bond and thus cannot be chilled forcefully by an external person."]
                        CannotChillOther,
                        #[codec(index = 21)]
                        #[doc = "There are too many nominators in the system. Governance needs to adjust the staking"]
                        #[doc = "settings to keep things safe for the runtime."]
                        TooManyNominators,
                        #[codec(index = 22)]
                        #[doc = "There are too many validator candidates in the system. Governance needs to adjust the"]
                        #[doc = "staking settings to keep things safe for the runtime."]
                        TooManyValidators,
                        #[codec(index = 23)]
                        #[doc = "Commission is too low. Must be at least `MinCommission`."]
                        CommissionTooLow,
                        #[codec(index = 24)]
                        #[doc = "Some bound is not met."]
                        BoundNotMet,
                    }
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
                    pub enum Event {
                        #[codec(index = 0)]
                        #[doc = "The era payout has been set; the first balance is the validator-payout; the second is"]
                        #[doc = "the remainder from the maximum amount of reward."]
                        EraPaid {
                            era_index: ::core::primitive::u32,
                            validator_payout: ::core::primitive::u128,
                            remainder: ::core::primitive::u128,
                        },
                        #[codec(index = 1)]
                        #[doc = "The nominator has been rewarded by this amount."]
                        Rewarded {
                            stash: ::subxt::ext::sp_core::crypto::AccountId32,
                            amount: ::core::primitive::u128,
                        },
                        #[codec(index = 2)]
                        #[doc = "A staker (validator or nominator) has been slashed by the given amount."]
                        Slashed {
                            staker: ::subxt::ext::sp_core::crypto::AccountId32,
                            amount: ::core::primitive::u128,
                        },
                        #[codec(index = 3)]
                        #[doc = "A slash for the given validator, for the given percentage of their stake, at the given"]
                        #[doc = "era as been reported."]
                        SlashReported {
                            validator: ::subxt::ext::sp_core::crypto::AccountId32,
                            fraction: runtime_types::sp_arithmetic::per_things::Perbill,
                            slash_era: ::core::primitive::u32,
                        },
                        #[codec(index = 4)]
                        #[doc = "An old slashing report from a prior era was discarded because it could"]
                        #[doc = "not be processed."]
                        OldSlashingReportDiscarded {
                            session_index: ::core::primitive::u32,
                        },
                        #[codec(index = 5)]
                        #[doc = "A new set of stakers was elected."]
                        StakersElected,
                        #[codec(index = 6)]
                        #[doc = "An account has bonded this amount. \\[stash, amount\\]"]
                        #[doc = ""]
                        #[doc = "NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,"]
                        #[doc = "it will not be emitted for staking rewards when they are added to stake."]
                        Bonded {
                            stash: ::subxt::ext::sp_core::crypto::AccountId32,
                            amount: ::core::primitive::u128,
                        },
                        #[codec(index = 7)]
                        #[doc = "An account has unbonded this amount."]
                        Unbonded {
                            stash: ::subxt::ext::sp_core::crypto::AccountId32,
                            amount: ::core::primitive::u128,
                        },
                        #[codec(index = 8)]
                        #[doc = "An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`"]
                        #[doc = "from the unlocking queue."]
                        Withdrawn {
                            stash: ::subxt::ext::sp_core::crypto::AccountId32,
                            amount: ::core::primitive::u128,
                        },
                        #[codec(index = 9)]
                        #[doc = "A nominator has been kicked from a validator."]
                        Kicked {
                            nominator: ::subxt::ext::sp_core::crypto::AccountId32,
                            stash: ::subxt::ext::sp_core::crypto::AccountId32,
                        },
                        #[codec(index = 10)]
                        #[doc = "The election failed. No new era is planned."]
                        StakingElectionFailed,
                        #[codec(index = 11)]
                        #[doc = "An account has stopped participating as either a validator or nominator."]
                        Chilled {
                            stash: ::subxt::ext::sp_core::crypto::AccountId32,
                        },
                        #[codec(index = 12)]
                        #[doc = "The stakers' rewards are getting paid."]
                        PayoutStarted {
                            era_index: ::core::primitive::u32,
                            validator_stash: ::subxt::ext::sp_core::crypto::AccountId32,
                        },
                        #[codec(index = 13)]
                        #[doc = "A validator has set their preferences."]
                        ValidatorPrefsSet {
                            stash: ::subxt::ext::sp_core::crypto::AccountId32,
                            prefs: runtime_types::pallet_staking::ValidatorPrefs,
                        },
                        #[codec(index = 14)]
                        #[doc = "A new force era mode was set."]
                        ForceEra {
                            mode: runtime_types::pallet_staking::Forcing,
                        },
                    }
                }
            }
            pub mod slashing {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct SlashingSpans {
                    pub span_index: ::core::primitive::u32,
                    pub last_start: ::core::primitive::u32,
                    pub last_nonzero_slash: ::core::primitive::u32,
                    pub prior: ::std::vec::Vec<::core::primitive::u32>,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct SpanRecord<_0> {
                    pub slashed: _0,
                    pub paid_out: _0,
                }
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ActiveEraInfo {
                pub index: ::core::primitive::u32,
                pub start: ::core::option::Option<::core::primitive::u64>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct EraRewardPoints<_0> {
                pub total: ::core::primitive::u32,
                pub individual: ::subxt::utils::KeyedVec<_0, ::core::primitive::u32>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Exposure<_0, _1> {
                #[codec(compact)]
                pub total: _1,
                #[codec(compact)]
                pub own: _1,
                pub others:
                    ::std::vec::Vec<runtime_types::pallet_staking::IndividualExposure<_0, _1>>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum Forcing {
                #[codec(index = 0)]
                NotForcing,
                #[codec(index = 1)]
                ForceNew,
                #[codec(index = 2)]
                ForceNone,
                #[codec(index = 3)]
                ForceAlways,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct IndividualExposure<_0, _1> {
                pub who: _0,
                #[codec(compact)]
                pub value: _1,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Nominations {
                pub targets: runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                    ::subxt::ext::sp_core::crypto::AccountId32,
                >,
                pub submitted_in: ::core::primitive::u32,
                pub suppressed: ::core::primitive::bool,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum RewardDestination<_0> {
                #[codec(index = 0)]
                Staked,
                #[codec(index = 1)]
                Stash,
                #[codec(index = 2)]
                Controller,
                #[codec(index = 3)]
                Account(_0),
                #[codec(index = 4)]
                None,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct StakingLedger {
                pub stash: ::subxt::ext::sp_core::crypto::AccountId32,
                #[codec(compact)]
                pub total: ::core::primitive::u128,
                #[codec(compact)]
                pub active: ::core::primitive::u128,
                pub unlocking: runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                    runtime_types::pallet_staking::UnlockChunk<::core::primitive::u128>,
                >,
                pub claimed_rewards: runtime_types::sp_core::bounded::bounded_vec::BoundedVec<
                    ::core::primitive::u32,
                >,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct UnappliedSlash<_0, _1> {
                pub validator: _0,
                pub own: _1,
                pub others: ::std::vec::Vec<(_0, _1)>,
                pub reporters: ::std::vec::Vec<_0>,
                pub payout: _1,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct UnlockChunk<_0> {
                #[codec(compact)]
                pub value: _0,
                #[codec(compact)]
                pub era: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ValidatorPrefs {
                #[codec(compact)]
                pub commission: runtime_types::sp_arithmetic::per_things::Perbill,
                pub blocked: ::core::primitive::bool,
            }
        }
        pub mod pallet_sudo {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
                pub enum Call {
                    #[codec(index = 0)]
                    #[doc = "Authenticates the sudo key and dispatches a function call with `Root` origin."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- O(1)."]
                    #[doc = "- Limited storage reads."]
                    #[doc = "- One DB write (event)."]
                    #[doc = "- Weight of derivative `call` execution + 10,000."]
                    #[doc = "# </weight>"]
                    sudo {
                        call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
                    },
                    #[codec(index = 1)]
                    #[doc = "Authenticates the sudo key and dispatches a function call with `Root` origin."]
                    #[doc = "This function does not check the weight of the call, and instead allows the"]
                    #[doc = "Sudo user to specify the weight of the call."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- O(1)."]
                    #[doc = "- The weight of this call is defined by the caller."]
                    #[doc = "# </weight>"]
                    sudo_unchecked_weight {
                        call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
                        weight: runtime_types::sp_weights::weight_v2::Weight,
                    },
                    #[codec(index = 2)]
                    #[doc = "Authenticates the current sudo key and sets the given AccountId (`new`) as the new sudo"]
                    #[doc = "key."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- O(1)."]
                    #[doc = "- Limited storage reads."]
                    #[doc = "- One DB change."]
                    #[doc = "# </weight>"]
                    set_key {
                        new: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                    },
                    #[codec(index = 3)]
                    #[doc = "Authenticates the sudo key and dispatches a function call with `Signed` origin from"]
                    #[doc = "a given account."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- O(1)."]
                    #[doc = "- Limited storage reads."]
                    #[doc = "- One DB write (event)."]
                    #[doc = "- Weight of derivative `call` execution + 10,000."]
                    #[doc = "# </weight>"]
                    sudo_as {
                        who: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
                    },
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Error for the Sudo pallet"]
                pub enum Error {
                    #[codec(index = 0)]
                    #[doc = "Sender must be the Sudo account"]
                    RequireSudo,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
                pub enum Event {
                    #[codec(index = 0)]
                    #[doc = "A sudo just took place. \\[result\\]"]
                    Sudid {
                        sudo_result:
                            ::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
                    },
                    #[codec(index = 1)]
                    #[doc = "The \\[sudoer\\] just switched identity; the old key is supplied if one existed."]
                    KeyChanged {
                        old_sudoer:
                            ::core::option::Option<::subxt::ext::sp_core::crypto::AccountId32>,
                    },
                    #[codec(index = 2)]
                    #[doc = "A sudo just took place. \\[result\\]"]
                    SudoAsDone {
                        sudo_result:
                            ::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
                    },
                }
            }
        }
        pub mod pallet_timestamp {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
                pub enum Call {
                    #[codec(index = 0)]
                    #[doc = "Set the current time."]
                    #[doc = ""]
                    #[doc = "This call should be invoked exactly once per block. It will panic at the finalization"]
                    #[doc = "phase, if this call hasn't been invoked by that time."]
                    #[doc = ""]
                    #[doc = "The timestamp should be greater than the previous one by the amount specified by"]
                    #[doc = "`MinimumPeriod`."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be `Inherent`."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(1)` (Note that implementations of `OnTimestampSet` must also be `O(1)`)"]
                    #[doc = "- 1 storage read and 1 storage mutation (codec `O(1)`). (because of `DidUpdate::take` in"]
                    #[doc = "  `on_finalize`)"]
                    #[doc = "- 1 event handler `on_timestamp_set`. Must be `O(1)`."]
                    #[doc = "# </weight>"]
                    set {
                        #[codec(compact)]
                        now: ::core::primitive::u64,
                    },
                }
            }
        }
        pub mod pallet_transaction_payment {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
                pub enum Event {
                    #[codec(index = 0)]
                    #[doc = "A transaction fee `actual_fee`, of which `tip` was added to the minimum inclusion fee,"]
                    #[doc = "has been paid by `who`."]
                    TransactionFeePaid {
                        who: ::subxt::ext::sp_core::crypto::AccountId32,
                        actual_fee: ::core::primitive::u128,
                        tip: ::core::primitive::u128,
                    },
                }
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ChargeTransactionPayment(#[codec(compact)] pub ::core::primitive::u128);
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum Releases {
                #[codec(index = 0)]
                V1Ancient,
                #[codec(index = 1)]
                V2,
            }
        }
        pub mod pallet_treasury {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
                pub enum Call {
                    #[codec(index = 0)]
                    #[doc = "Put forward a suggestion for spending. A deposit proportional to the value"]
                    #[doc = "is reserved and slashed if the proposal is rejected. It is returned once the"]
                    #[doc = "proposal is awarded."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- Complexity: O(1)"]
                    #[doc = "- DbReads: `ProposalCount`, `origin account`"]
                    #[doc = "- DbWrites: `ProposalCount`, `Proposals`, `origin account`"]
                    #[doc = "# </weight>"]
                    propose_spend {
                        #[codec(compact)]
                        value: ::core::primitive::u128,
                        beneficiary: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                    },
                    #[codec(index = 1)]
                    #[doc = "Reject a proposed spend. The original deposit will be slashed."]
                    #[doc = ""]
                    #[doc = "May only be called from `T::RejectOrigin`."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- Complexity: O(1)"]
                    #[doc = "- DbReads: `Proposals`, `rejected proposer account`"]
                    #[doc = "- DbWrites: `Proposals`, `rejected proposer account`"]
                    #[doc = "# </weight>"]
                    reject_proposal {
                        #[codec(compact)]
                        proposal_id: ::core::primitive::u32,
                    },
                    #[codec(index = 2)]
                    #[doc = "Approve a proposal. At a later time, the proposal will be allocated to the beneficiary"]
                    #[doc = "and the original deposit will be returned."]
                    #[doc = ""]
                    #[doc = "May only be called from `T::ApproveOrigin`."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- Complexity: O(1)."]
                    #[doc = "- DbReads: `Proposals`, `Approvals`"]
                    #[doc = "- DbWrite: `Approvals`"]
                    #[doc = "# </weight>"]
                    approve_proposal {
                        #[codec(compact)]
                        proposal_id: ::core::primitive::u32,
                    },
                    #[codec(index = 3)]
                    #[doc = "Propose and approve a spend of treasury funds."]
                    #[doc = ""]
                    #[doc = "- `origin`: Must be `SpendOrigin` with the `Success` value being at least `amount`."]
                    #[doc = "- `amount`: The amount to be transferred from the treasury to the `beneficiary`."]
                    #[doc = "- `beneficiary`: The destination account for the transfer."]
                    #[doc = ""]
                    #[doc = "NOTE: For record-keeping purposes, the proposer is deemed to be equivalent to the"]
                    #[doc = "beneficiary."]
                    spend {
                        #[codec(compact)]
                        amount: ::core::primitive::u128,
                        beneficiary: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                    },
                    #[codec(index = 4)]
                    #[doc = "Force a previously approved proposal to be removed from the approval queue."]
                    #[doc = "The original deposit will no longer be returned."]
                    #[doc = ""]
                    #[doc = "May only be called from `T::RejectOrigin`."]
                    #[doc = "- `proposal_id`: The index of a proposal"]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- Complexity: O(A) where `A` is the number of approvals"]
                    #[doc = "- Db reads and writes: `Approvals`"]
                    #[doc = "# </weight>"]
                    #[doc = ""]
                    #[doc = "Errors:"]
                    #[doc = "- `ProposalNotApproved`: The `proposal_id` supplied was not found in the approval queue,"]
                    #[doc = "i.e., the proposal has not been approved. This could also mean the proposal does not"]
                    #[doc = "exist altogether, thus there is no way it would have been approved in the first place."]
                    remove_approval {
                        #[codec(compact)]
                        proposal_id: ::core::primitive::u32,
                    },
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Error for the treasury pallet."]
                pub enum Error {
                    #[codec(index = 0)]
                    #[doc = "Proposer's balance is too low."]
                    InsufficientProposersBalance,
                    #[codec(index = 1)]
                    #[doc = "No proposal or bounty at that index."]
                    InvalidIndex,
                    #[codec(index = 2)]
                    #[doc = "Too many approvals in the queue."]
                    TooManyApprovals,
                    #[codec(index = 3)]
                    #[doc = "The spend origin is valid but the amount it is allowed to spend is lower than the"]
                    #[doc = "amount to be spent."]
                    InsufficientPermission,
                    #[codec(index = 4)]
                    #[doc = "Proposal has not been approved."]
                    ProposalNotApproved,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
                pub enum Event {
                    #[codec(index = 0)]
                    #[doc = "New proposal."]
                    Proposed {
                        proposal_index: ::core::primitive::u32,
                    },
                    #[codec(index = 1)]
                    #[doc = "We have ended a spend period and will now allocate funds."]
                    Spending {
                        budget_remaining: ::core::primitive::u128,
                    },
                    #[codec(index = 2)]
                    #[doc = "Some funds have been allocated."]
                    Awarded {
                        proposal_index: ::core::primitive::u32,
                        award: ::core::primitive::u128,
                        account: ::subxt::ext::sp_core::crypto::AccountId32,
                    },
                    #[codec(index = 3)]
                    #[doc = "A proposal was rejected; funds were slashed."]
                    Rejected {
                        proposal_index: ::core::primitive::u32,
                        slashed: ::core::primitive::u128,
                    },
                    #[codec(index = 4)]
                    #[doc = "Some of our funds have been burnt."]
                    Burnt {
                        burnt_funds: ::core::primitive::u128,
                    },
                    #[codec(index = 5)]
                    #[doc = "Spending has finished; this is the amount that rolls over until next spend."]
                    Rollover {
                        rollover_balance: ::core::primitive::u128,
                    },
                    #[codec(index = 6)]
                    #[doc = "Some funds have been deposited."]
                    Deposit { value: ::core::primitive::u128 },
                    #[codec(index = 7)]
                    #[doc = "A new spend proposal has been approved."]
                    SpendApproved {
                        proposal_index: ::core::primitive::u32,
                        amount: ::core::primitive::u128,
                        beneficiary: ::subxt::ext::sp_core::crypto::AccountId32,
                    },
                    #[codec(index = 8)]
                    #[doc = "The inactive funds of the pallet have been updated."]
                    UpdatedInactive {
                        reactivated: ::core::primitive::u128,
                        deactivated: ::core::primitive::u128,
                    },
                }
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Proposal<_0, _1> {
                pub proposer: _0,
                pub value: _1,
                pub beneficiary: _0,
                pub bond: _1,
            }
        }
        pub mod pallet_utility {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
                pub enum Call {
                    #[codec(index = 0)]
                    #[doc = "Send a batch of dispatch calls."]
                    #[doc = ""]
                    #[doc = "May be called from any origin except `None`."]
                    #[doc = ""]
                    #[doc = "- `calls`: The calls to be dispatched from the same origin. The number of call must not"]
                    #[doc = "  exceed the constant: `batched_calls_limit` (available in constant metadata)."]
                    #[doc = ""]
                    #[doc = "If origin is root then the calls are dispatched without checking origin filter. (This"]
                    #[doc = "includes bypassing `frame_system::Config::BaseCallFilter`)."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- Complexity: O(C) where C is the number of calls to be batched."]
                    #[doc = "# </weight>"]
                    #[doc = ""]
                    #[doc = "This will return `Ok` in all circumstances. To determine the success of the batch, an"]
                    #[doc = "event is deposited. If a call failed and the batch was interrupted, then the"]
                    #[doc = "`BatchInterrupted` event is deposited, along with the number of successful calls made"]
                    #[doc = "and the error of the failed call. If all were successful, then the `BatchCompleted`"]
                    #[doc = "event is deposited."]
                    batch {
                        calls: ::std::vec::Vec<runtime_types::aleph_runtime::RuntimeCall>,
                    },
                    #[codec(index = 1)]
                    #[doc = "Send a call through an indexed pseudonym of the sender."]
                    #[doc = ""]
                    #[doc = "Filter from origin are passed along. The call will be dispatched with an origin which"]
                    #[doc = "use the same filter as the origin of this call."]
                    #[doc = ""]
                    #[doc = "NOTE: If you need to ensure that any account-based filtering is not honored (i.e."]
                    #[doc = "because you expect `proxy` to have been used prior in the call stack and you do not want"]
                    #[doc = "the call restrictions to apply to any sub-accounts), then use `as_multi_threshold_1`"]
                    #[doc = "in the Multisig pallet instead."]
                    #[doc = ""]
                    #[doc = "NOTE: Prior to version *12, this was called `as_limited_sub`."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_."]
                    as_derivative {
                        index: ::core::primitive::u16,
                        call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
                    },
                    #[codec(index = 2)]
                    #[doc = "Send a batch of dispatch calls and atomically execute them."]
                    #[doc = "The whole transaction will rollback and fail if any of the calls failed."]
                    #[doc = ""]
                    #[doc = "May be called from any origin except `None`."]
                    #[doc = ""]
                    #[doc = "- `calls`: The calls to be dispatched from the same origin. The number of call must not"]
                    #[doc = "  exceed the constant: `batched_calls_limit` (available in constant metadata)."]
                    #[doc = ""]
                    #[doc = "If origin is root then the calls are dispatched without checking origin filter. (This"]
                    #[doc = "includes bypassing `frame_system::Config::BaseCallFilter`)."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- Complexity: O(C) where C is the number of calls to be batched."]
                    #[doc = "# </weight>"]
                    batch_all {
                        calls: ::std::vec::Vec<runtime_types::aleph_runtime::RuntimeCall>,
                    },
                    #[codec(index = 3)]
                    #[doc = "Dispatches a function call with a provided origin."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Root_."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- O(1)."]
                    #[doc = "- Limited storage reads."]
                    #[doc = "- One DB write (event)."]
                    #[doc = "- Weight of derivative `call` execution + T::WeightInfo::dispatch_as()."]
                    #[doc = "# </weight>"]
                    dispatch_as {
                        as_origin: ::std::boxed::Box<runtime_types::aleph_runtime::OriginCaller>,
                        call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
                    },
                    #[codec(index = 4)]
                    #[doc = "Send a batch of dispatch calls."]
                    #[doc = "Unlike `batch`, it allows errors and won't interrupt."]
                    #[doc = ""]
                    #[doc = "May be called from any origin except `None`."]
                    #[doc = ""]
                    #[doc = "- `calls`: The calls to be dispatched from the same origin. The number of call must not"]
                    #[doc = "  exceed the constant: `batched_calls_limit` (available in constant metadata)."]
                    #[doc = ""]
                    #[doc = "If origin is root then the calls are dispatch without checking origin filter. (This"]
                    #[doc = "includes bypassing `frame_system::Config::BaseCallFilter`)."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- Complexity: O(C) where C is the number of calls to be batched."]
                    #[doc = "# </weight>"]
                    force_batch {
                        calls: ::std::vec::Vec<runtime_types::aleph_runtime::RuntimeCall>,
                    },
                    #[codec(index = 5)]
                    #[doc = "Dispatch a function call with a specified weight."]
                    #[doc = ""]
                    #[doc = "This function does not check the weight of the call, and instead allows the"]
                    #[doc = "Root origin to specify the weight of the call."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Root_."]
                    with_weight {
                        call: ::std::boxed::Box<runtime_types::aleph_runtime::RuntimeCall>,
                        weight: runtime_types::sp_weights::weight_v2::Weight,
                    },
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tCustom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)\n\t\t\tof this pallet.\n\t\t\t"]
                pub enum Error {
                    #[codec(index = 0)]
                    #[doc = "Too many calls batched."]
                    TooManyCalls,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
                pub enum Event {
                    #[codec(index = 0)]
                    #[doc = "Batch of dispatches did not complete fully. Index of first failing dispatch given, as"]
                    #[doc = "well as the error."]
                    BatchInterrupted {
                        index: ::core::primitive::u32,
                        error: runtime_types::sp_runtime::DispatchError,
                    },
                    #[codec(index = 1)]
                    #[doc = "Batch of dispatches completed fully with no error."]
                    BatchCompleted,
                    #[codec(index = 2)]
                    #[doc = "Batch of dispatches completed but has errors."]
                    BatchCompletedWithErrors,
                    #[codec(index = 3)]
                    #[doc = "A single item within a Batch of dispatches has completed with no error."]
                    ItemCompleted,
                    #[codec(index = 4)]
                    #[doc = "A single item within a Batch of dispatches has completed with error."]
                    ItemFailed {
                        error: runtime_types::sp_runtime::DispatchError,
                    },
                    #[codec(index = 5)]
                    #[doc = "A call was dispatched."]
                    DispatchedAs {
                        result:
                            ::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
                    },
                }
            }
        }
        pub mod pallet_vesting {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Contains one variant per dispatchable that can be called by an extrinsic."]
                pub enum Call {
                    #[codec(index = 0)]
                    #[doc = "Unlock any vested funds of the sender account."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_ and the sender must have funds still"]
                    #[doc = "locked under this pallet."]
                    #[doc = ""]
                    #[doc = "Emits either `VestingCompleted` or `VestingUpdated`."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(1)`."]
                    #[doc = "- DbWeight: 2 Reads, 2 Writes"]
                    #[doc = "    - Reads: Vesting Storage, Balances Locks, [Sender Account]"]
                    #[doc = "    - Writes: Vesting Storage, Balances Locks, [Sender Account]"]
                    #[doc = "# </weight>"]
                    vest,
                    #[codec(index = 1)]
                    #[doc = "Unlock any vested funds of a `target` account."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_."]
                    #[doc = ""]
                    #[doc = "- `target`: The account whose vested funds should be unlocked. Must have funds still"]
                    #[doc = "locked under this pallet."]
                    #[doc = ""]
                    #[doc = "Emits either `VestingCompleted` or `VestingUpdated`."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(1)`."]
                    #[doc = "- DbWeight: 3 Reads, 3 Writes"]
                    #[doc = "    - Reads: Vesting Storage, Balances Locks, Target Account"]
                    #[doc = "    - Writes: Vesting Storage, Balances Locks, Target Account"]
                    #[doc = "# </weight>"]
                    vest_other {
                        target: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                    },
                    #[codec(index = 2)]
                    #[doc = "Create a vested transfer."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_."]
                    #[doc = ""]
                    #[doc = "- `target`: The account receiving the vested funds."]
                    #[doc = "- `schedule`: The vesting schedule attached to the transfer."]
                    #[doc = ""]
                    #[doc = "Emits `VestingCreated`."]
                    #[doc = ""]
                    #[doc = "NOTE: This will unlock all schedules through the current block."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(1)`."]
                    #[doc = "- DbWeight: 3 Reads, 3 Writes"]
                    #[doc = "    - Reads: Vesting Storage, Balances Locks, Target Account, [Sender Account]"]
                    #[doc = "    - Writes: Vesting Storage, Balances Locks, Target Account, [Sender Account]"]
                    #[doc = "# </weight>"]
                    vested_transfer {
                        target: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        schedule: runtime_types::pallet_vesting::vesting_info::VestingInfo<
                            ::core::primitive::u128,
                            ::core::primitive::u32,
                        >,
                    },
                    #[codec(index = 3)]
                    #[doc = "Force a vested transfer."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Root_."]
                    #[doc = ""]
                    #[doc = "- `source`: The account whose funds should be transferred."]
                    #[doc = "- `target`: The account that should be transferred the vested funds."]
                    #[doc = "- `schedule`: The vesting schedule attached to the transfer."]
                    #[doc = ""]
                    #[doc = "Emits `VestingCreated`."]
                    #[doc = ""]
                    #[doc = "NOTE: This will unlock all schedules through the current block."]
                    #[doc = ""]
                    #[doc = "# <weight>"]
                    #[doc = "- `O(1)`."]
                    #[doc = "- DbWeight: 4 Reads, 4 Writes"]
                    #[doc = "    - Reads: Vesting Storage, Balances Locks, Target Account, Source Account"]
                    #[doc = "    - Writes: Vesting Storage, Balances Locks, Target Account, Source Account"]
                    #[doc = "# </weight>"]
                    force_vested_transfer {
                        source: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        target: ::subxt::ext::sp_runtime::MultiAddress<
                            ::subxt::ext::sp_core::crypto::AccountId32,
                            (),
                        >,
                        schedule: runtime_types::pallet_vesting::vesting_info::VestingInfo<
                            ::core::primitive::u128,
                            ::core::primitive::u32,
                        >,
                    },
                    #[codec(index = 4)]
                    #[doc = "Merge two vesting schedules together, creating a new vesting schedule that unlocks over"]
                    #[doc = "the highest possible start and end blocks. If both schedules have already started the"]
                    #[doc = "current block will be used as the schedule start; with the caveat that if one schedule"]
                    #[doc = "is finished by the current block, the other will be treated as the new merged schedule,"]
                    #[doc = "unmodified."]
                    #[doc = ""]
                    #[doc = "NOTE: If `schedule1_index == schedule2_index` this is a no-op."]
                    #[doc = "NOTE: This will unlock all schedules through the current block prior to merging."]
                    #[doc = "NOTE: If both schedules have ended by the current block, no new schedule will be created"]
                    #[doc = "and both will be removed."]
                    #[doc = ""]
                    #[doc = "Merged schedule attributes:"]
                    #[doc = "- `starting_block`: `MAX(schedule1.starting_block, scheduled2.starting_block,"]
                    #[doc = "  current_block)`."]
                    #[doc = "- `ending_block`: `MAX(schedule1.ending_block, schedule2.ending_block)`."]
                    #[doc = "- `locked`: `schedule1.locked_at(current_block) + schedule2.locked_at(current_block)`."]
                    #[doc = ""]
                    #[doc = "The dispatch origin for this call must be _Signed_."]
                    #[doc = ""]
                    #[doc = "- `schedule1_index`: index of the first schedule to merge."]
                    #[doc = "- `schedule2_index`: index of the second schedule to merge."]
                    merge_schedules {
                        schedule1_index: ::core::primitive::u32,
                        schedule2_index: ::core::primitive::u32,
                    },
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "Error for the vesting pallet."]
                pub enum Error {
                    #[codec(index = 0)]
                    #[doc = "The account given is not vesting."]
                    NotVesting,
                    #[codec(index = 1)]
                    #[doc = "The account already has `MaxVestingSchedules` count of schedules and thus"]
                    #[doc = "cannot add another one. Consider merging existing schedules in order to add another."]
                    AtMaxVestingSchedules,
                    #[codec(index = 2)]
                    #[doc = "Amount being transferred is too low to create a vesting schedule."]
                    AmountLow,
                    #[codec(index = 3)]
                    #[doc = "An index was out of bounds of the vesting schedules."]
                    ScheduleIndexOutOfBounds,
                    #[codec(index = 4)]
                    #[doc = "Failed to create a new schedule because some parameter was invalid."]
                    InvalidScheduleParams,
                }
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                #[doc = "\n\t\t\tThe [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted\n\t\t\tby this pallet.\n\t\t\t"]
                pub enum Event {
                    #[codec(index = 0)]
                    #[doc = "The amount vested has been updated. This could indicate a change in funds available."]
                    #[doc = "The balance given is the amount which is left unvested (and thus locked)."]
                    VestingUpdated {
                        account: ::subxt::ext::sp_core::crypto::AccountId32,
                        unvested: ::core::primitive::u128,
                    },
                    #[codec(index = 1)]
                    #[doc = "An \\[account\\] has become fully vested."]
                    VestingCompleted {
                        account: ::subxt::ext::sp_core::crypto::AccountId32,
                    },
                }
            }
            pub mod vesting_info {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct VestingInfo<_0, _1> {
                    pub locked: _0,
                    pub per_block: _0,
                    pub starting_block: _1,
                }
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum Releases {
                #[codec(index = 0)]
                V0,
                #[codec(index = 1)]
                V1,
            }
        }
        pub mod primitive_types {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct H256(pub [::core::primitive::u8; 32usize]);
        }
        pub mod primitives {
            use super::runtime_types;
            pub mod app {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct Public(pub runtime_types::sp_core::ed25519::Public);
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct BanConfig {
                pub minimal_expected_performance: runtime_types::sp_arithmetic::per_things::Perbill,
                pub underperformed_session_count_threshold: ::core::primitive::u32,
                pub clean_session_counter_delay: ::core::primitive::u32,
                pub ban_period: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct BanInfo {
                pub reason: runtime_types::primitives::BanReason,
                pub start: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum BanReason {
                #[codec(index = 0)]
                InsufficientUptime(::core::primitive::u32),
                #[codec(index = 1)]
                OtherReason(
                    runtime_types::sp_core::bounded::bounded_vec::BoundedVec<::core::primitive::u8>,
                ),
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct CommitteeSeats {
                pub reserved_seats: ::core::primitive::u32,
                pub non_reserved_seats: ::core::primitive::u32,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum ElectionOpenness {
                #[codec(index = 0)]
                Permissioned,
                #[codec(index = 1)]
                Permissionless,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct EraValidators<_0> {
                pub reserved: ::std::vec::Vec<_0>,
                pub non_reserved: ::std::vec::Vec<_0>,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct VersionChange {
                pub version_incoming: ::core::primitive::u32,
                pub session: ::core::primitive::u32,
            }
        }
        pub mod sp_arithmetic {
            use super::runtime_types;
            pub mod fixed_point {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: CompactAs,
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct FixedU128(pub ::core::primitive::u128);
            }
            pub mod per_things {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: CompactAs,
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct Perbill(pub ::core::primitive::u32);
                #[derive(
                    :: subxt :: ext :: codec :: CompactAs,
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct Percent(pub ::core::primitive::u8);
                #[derive(
                    :: subxt :: ext :: codec :: CompactAs,
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct Permill(pub ::core::primitive::u32);
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum ArithmeticError {
                #[codec(index = 0)]
                Underflow,
                #[codec(index = 1)]
                Overflow,
                #[codec(index = 2)]
                DivisionByZero,
            }
        }
        pub mod sp_consensus_aura {
            use super::runtime_types;
            pub mod sr25519 {
                use super::runtime_types;
                pub mod app_sr25519 {
                    use super::runtime_types;
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    pub struct Public(pub runtime_types::sp_core::sr25519::Public);
                }
            }
        }
        pub mod sp_consensus_slots {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: CompactAs,
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct Slot(pub ::core::primitive::u64);
        }
        pub mod sp_core {
            use super::runtime_types;
            pub mod bounded {
                use super::runtime_types;
                pub mod bounded_btree_map {
                    use super::runtime_types;
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    pub struct BoundedBTreeMap<_0, _1>(pub ::subxt::utils::KeyedVec<_0, _1>);
                }
                pub mod bounded_vec {
                    use super::runtime_types;
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    pub struct BoundedVec<_0>(pub ::std::vec::Vec<_0>);
                }
                pub mod weak_bounded_vec {
                    use super::runtime_types;
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    pub struct WeakBoundedVec<_0>(pub ::std::vec::Vec<_0>);
                }
            }
            pub mod crypto {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct AccountId32(pub [::core::primitive::u8; 32usize]);
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct KeyTypeId(pub [::core::primitive::u8; 4usize]);
            }
            pub mod ecdsa {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct Signature(pub [::core::primitive::u8; 65usize]);
            }
            pub mod ed25519 {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct Public(pub [::core::primitive::u8; 32usize]);
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct Signature(pub [::core::primitive::u8; 64usize]);
            }
            pub mod sr25519 {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct Public(pub [::core::primitive::u8; 32usize]);
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct Signature(pub [::core::primitive::u8; 64usize]);
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum Void {}
        }
        pub mod sp_runtime {
            use super::runtime_types;
            pub mod generic {
                use super::runtime_types;
                pub mod digest {
                    use super::runtime_types;
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    pub struct Digest {
                        pub logs:
                            ::std::vec::Vec<runtime_types::sp_runtime::generic::digest::DigestItem>,
                    }
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    pub enum DigestItem {
                        #[codec(index = 6)]
                        PreRuntime(
                            [::core::primitive::u8; 4usize],
                            ::std::vec::Vec<::core::primitive::u8>,
                        ),
                        #[codec(index = 4)]
                        Consensus(
                            [::core::primitive::u8; 4usize],
                            ::std::vec::Vec<::core::primitive::u8>,
                        ),
                        #[codec(index = 5)]
                        Seal(
                            [::core::primitive::u8; 4usize],
                            ::std::vec::Vec<::core::primitive::u8>,
                        ),
                        #[codec(index = 0)]
                        Other(::std::vec::Vec<::core::primitive::u8>),
                        #[codec(index = 8)]
                        RuntimeEnvironmentUpdated,
                    }
                }
                pub mod era {
                    use super::runtime_types;
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    pub enum Era {
                        #[codec(index = 0)]
                        Immortal,
                        #[codec(index = 1)]
                        Mortal1(::core::primitive::u8),
                        #[codec(index = 2)]
                        Mortal2(::core::primitive::u8),
                        #[codec(index = 3)]
                        Mortal3(::core::primitive::u8),
                        #[codec(index = 4)]
                        Mortal4(::core::primitive::u8),
                        #[codec(index = 5)]
                        Mortal5(::core::primitive::u8),
                        #[codec(index = 6)]
                        Mortal6(::core::primitive::u8),
                        #[codec(index = 7)]
                        Mortal7(::core::primitive::u8),
                        #[codec(index = 8)]
                        Mortal8(::core::primitive::u8),
                        #[codec(index = 9)]
                        Mortal9(::core::primitive::u8),
                        #[codec(index = 10)]
                        Mortal10(::core::primitive::u8),
                        #[codec(index = 11)]
                        Mortal11(::core::primitive::u8),
                        #[codec(index = 12)]
                        Mortal12(::core::primitive::u8),
                        #[codec(index = 13)]
                        Mortal13(::core::primitive::u8),
                        #[codec(index = 14)]
                        Mortal14(::core::primitive::u8),
                        #[codec(index = 15)]
                        Mortal15(::core::primitive::u8),
                        #[codec(index = 16)]
                        Mortal16(::core::primitive::u8),
                        #[codec(index = 17)]
                        Mortal17(::core::primitive::u8),
                        #[codec(index = 18)]
                        Mortal18(::core::primitive::u8),
                        #[codec(index = 19)]
                        Mortal19(::core::primitive::u8),
                        #[codec(index = 20)]
                        Mortal20(::core::primitive::u8),
                        #[codec(index = 21)]
                        Mortal21(::core::primitive::u8),
                        #[codec(index = 22)]
                        Mortal22(::core::primitive::u8),
                        #[codec(index = 23)]
                        Mortal23(::core::primitive::u8),
                        #[codec(index = 24)]
                        Mortal24(::core::primitive::u8),
                        #[codec(index = 25)]
                        Mortal25(::core::primitive::u8),
                        #[codec(index = 26)]
                        Mortal26(::core::primitive::u8),
                        #[codec(index = 27)]
                        Mortal27(::core::primitive::u8),
                        #[codec(index = 28)]
                        Mortal28(::core::primitive::u8),
                        #[codec(index = 29)]
                        Mortal29(::core::primitive::u8),
                        #[codec(index = 30)]
                        Mortal30(::core::primitive::u8),
                        #[codec(index = 31)]
                        Mortal31(::core::primitive::u8),
                        #[codec(index = 32)]
                        Mortal32(::core::primitive::u8),
                        #[codec(index = 33)]
                        Mortal33(::core::primitive::u8),
                        #[codec(index = 34)]
                        Mortal34(::core::primitive::u8),
                        #[codec(index = 35)]
                        Mortal35(::core::primitive::u8),
                        #[codec(index = 36)]
                        Mortal36(::core::primitive::u8),
                        #[codec(index = 37)]
                        Mortal37(::core::primitive::u8),
                        #[codec(index = 38)]
                        Mortal38(::core::primitive::u8),
                        #[codec(index = 39)]
                        Mortal39(::core::primitive::u8),
                        #[codec(index = 40)]
                        Mortal40(::core::primitive::u8),
                        #[codec(index = 41)]
                        Mortal41(::core::primitive::u8),
                        #[codec(index = 42)]
                        Mortal42(::core::primitive::u8),
                        #[codec(index = 43)]
                        Mortal43(::core::primitive::u8),
                        #[codec(index = 44)]
                        Mortal44(::core::primitive::u8),
                        #[codec(index = 45)]
                        Mortal45(::core::primitive::u8),
                        #[codec(index = 46)]
                        Mortal46(::core::primitive::u8),
                        #[codec(index = 47)]
                        Mortal47(::core::primitive::u8),
                        #[codec(index = 48)]
                        Mortal48(::core::primitive::u8),
                        #[codec(index = 49)]
                        Mortal49(::core::primitive::u8),
                        #[codec(index = 50)]
                        Mortal50(::core::primitive::u8),
                        #[codec(index = 51)]
                        Mortal51(::core::primitive::u8),
                        #[codec(index = 52)]
                        Mortal52(::core::primitive::u8),
                        #[codec(index = 53)]
                        Mortal53(::core::primitive::u8),
                        #[codec(index = 54)]
                        Mortal54(::core::primitive::u8),
                        #[codec(index = 55)]
                        Mortal55(::core::primitive::u8),
                        #[codec(index = 56)]
                        Mortal56(::core::primitive::u8),
                        #[codec(index = 57)]
                        Mortal57(::core::primitive::u8),
                        #[codec(index = 58)]
                        Mortal58(::core::primitive::u8),
                        #[codec(index = 59)]
                        Mortal59(::core::primitive::u8),
                        #[codec(index = 60)]
                        Mortal60(::core::primitive::u8),
                        #[codec(index = 61)]
                        Mortal61(::core::primitive::u8),
                        #[codec(index = 62)]
                        Mortal62(::core::primitive::u8),
                        #[codec(index = 63)]
                        Mortal63(::core::primitive::u8),
                        #[codec(index = 64)]
                        Mortal64(::core::primitive::u8),
                        #[codec(index = 65)]
                        Mortal65(::core::primitive::u8),
                        #[codec(index = 66)]
                        Mortal66(::core::primitive::u8),
                        #[codec(index = 67)]
                        Mortal67(::core::primitive::u8),
                        #[codec(index = 68)]
                        Mortal68(::core::primitive::u8),
                        #[codec(index = 69)]
                        Mortal69(::core::primitive::u8),
                        #[codec(index = 70)]
                        Mortal70(::core::primitive::u8),
                        #[codec(index = 71)]
                        Mortal71(::core::primitive::u8),
                        #[codec(index = 72)]
                        Mortal72(::core::primitive::u8),
                        #[codec(index = 73)]
                        Mortal73(::core::primitive::u8),
                        #[codec(index = 74)]
                        Mortal74(::core::primitive::u8),
                        #[codec(index = 75)]
                        Mortal75(::core::primitive::u8),
                        #[codec(index = 76)]
                        Mortal76(::core::primitive::u8),
                        #[codec(index = 77)]
                        Mortal77(::core::primitive::u8),
                        #[codec(index = 78)]
                        Mortal78(::core::primitive::u8),
                        #[codec(index = 79)]
                        Mortal79(::core::primitive::u8),
                        #[codec(index = 80)]
                        Mortal80(::core::primitive::u8),
                        #[codec(index = 81)]
                        Mortal81(::core::primitive::u8),
                        #[codec(index = 82)]
                        Mortal82(::core::primitive::u8),
                        #[codec(index = 83)]
                        Mortal83(::core::primitive::u8),
                        #[codec(index = 84)]
                        Mortal84(::core::primitive::u8),
                        #[codec(index = 85)]
                        Mortal85(::core::primitive::u8),
                        #[codec(index = 86)]
                        Mortal86(::core::primitive::u8),
                        #[codec(index = 87)]
                        Mortal87(::core::primitive::u8),
                        #[codec(index = 88)]
                        Mortal88(::core::primitive::u8),
                        #[codec(index = 89)]
                        Mortal89(::core::primitive::u8),
                        #[codec(index = 90)]
                        Mortal90(::core::primitive::u8),
                        #[codec(index = 91)]
                        Mortal91(::core::primitive::u8),
                        #[codec(index = 92)]
                        Mortal92(::core::primitive::u8),
                        #[codec(index = 93)]
                        Mortal93(::core::primitive::u8),
                        #[codec(index = 94)]
                        Mortal94(::core::primitive::u8),
                        #[codec(index = 95)]
                        Mortal95(::core::primitive::u8),
                        #[codec(index = 96)]
                        Mortal96(::core::primitive::u8),
                        #[codec(index = 97)]
                        Mortal97(::core::primitive::u8),
                        #[codec(index = 98)]
                        Mortal98(::core::primitive::u8),
                        #[codec(index = 99)]
                        Mortal99(::core::primitive::u8),
                        #[codec(index = 100)]
                        Mortal100(::core::primitive::u8),
                        #[codec(index = 101)]
                        Mortal101(::core::primitive::u8),
                        #[codec(index = 102)]
                        Mortal102(::core::primitive::u8),
                        #[codec(index = 103)]
                        Mortal103(::core::primitive::u8),
                        #[codec(index = 104)]
                        Mortal104(::core::primitive::u8),
                        #[codec(index = 105)]
                        Mortal105(::core::primitive::u8),
                        #[codec(index = 106)]
                        Mortal106(::core::primitive::u8),
                        #[codec(index = 107)]
                        Mortal107(::core::primitive::u8),
                        #[codec(index = 108)]
                        Mortal108(::core::primitive::u8),
                        #[codec(index = 109)]
                        Mortal109(::core::primitive::u8),
                        #[codec(index = 110)]
                        Mortal110(::core::primitive::u8),
                        #[codec(index = 111)]
                        Mortal111(::core::primitive::u8),
                        #[codec(index = 112)]
                        Mortal112(::core::primitive::u8),
                        #[codec(index = 113)]
                        Mortal113(::core::primitive::u8),
                        #[codec(index = 114)]
                        Mortal114(::core::primitive::u8),
                        #[codec(index = 115)]
                        Mortal115(::core::primitive::u8),
                        #[codec(index = 116)]
                        Mortal116(::core::primitive::u8),
                        #[codec(index = 117)]
                        Mortal117(::core::primitive::u8),
                        #[codec(index = 118)]
                        Mortal118(::core::primitive::u8),
                        #[codec(index = 119)]
                        Mortal119(::core::primitive::u8),
                        #[codec(index = 120)]
                        Mortal120(::core::primitive::u8),
                        #[codec(index = 121)]
                        Mortal121(::core::primitive::u8),
                        #[codec(index = 122)]
                        Mortal122(::core::primitive::u8),
                        #[codec(index = 123)]
                        Mortal123(::core::primitive::u8),
                        #[codec(index = 124)]
                        Mortal124(::core::primitive::u8),
                        #[codec(index = 125)]
                        Mortal125(::core::primitive::u8),
                        #[codec(index = 126)]
                        Mortal126(::core::primitive::u8),
                        #[codec(index = 127)]
                        Mortal127(::core::primitive::u8),
                        #[codec(index = 128)]
                        Mortal128(::core::primitive::u8),
                        #[codec(index = 129)]
                        Mortal129(::core::primitive::u8),
                        #[codec(index = 130)]
                        Mortal130(::core::primitive::u8),
                        #[codec(index = 131)]
                        Mortal131(::core::primitive::u8),
                        #[codec(index = 132)]
                        Mortal132(::core::primitive::u8),
                        #[codec(index = 133)]
                        Mortal133(::core::primitive::u8),
                        #[codec(index = 134)]
                        Mortal134(::core::primitive::u8),
                        #[codec(index = 135)]
                        Mortal135(::core::primitive::u8),
                        #[codec(index = 136)]
                        Mortal136(::core::primitive::u8),
                        #[codec(index = 137)]
                        Mortal137(::core::primitive::u8),
                        #[codec(index = 138)]
                        Mortal138(::core::primitive::u8),
                        #[codec(index = 139)]
                        Mortal139(::core::primitive::u8),
                        #[codec(index = 140)]
                        Mortal140(::core::primitive::u8),
                        #[codec(index = 141)]
                        Mortal141(::core::primitive::u8),
                        #[codec(index = 142)]
                        Mortal142(::core::primitive::u8),
                        #[codec(index = 143)]
                        Mortal143(::core::primitive::u8),
                        #[codec(index = 144)]
                        Mortal144(::core::primitive::u8),
                        #[codec(index = 145)]
                        Mortal145(::core::primitive::u8),
                        #[codec(index = 146)]
                        Mortal146(::core::primitive::u8),
                        #[codec(index = 147)]
                        Mortal147(::core::primitive::u8),
                        #[codec(index = 148)]
                        Mortal148(::core::primitive::u8),
                        #[codec(index = 149)]
                        Mortal149(::core::primitive::u8),
                        #[codec(index = 150)]
                        Mortal150(::core::primitive::u8),
                        #[codec(index = 151)]
                        Mortal151(::core::primitive::u8),
                        #[codec(index = 152)]
                        Mortal152(::core::primitive::u8),
                        #[codec(index = 153)]
                        Mortal153(::core::primitive::u8),
                        #[codec(index = 154)]
                        Mortal154(::core::primitive::u8),
                        #[codec(index = 155)]
                        Mortal155(::core::primitive::u8),
                        #[codec(index = 156)]
                        Mortal156(::core::primitive::u8),
                        #[codec(index = 157)]
                        Mortal157(::core::primitive::u8),
                        #[codec(index = 158)]
                        Mortal158(::core::primitive::u8),
                        #[codec(index = 159)]
                        Mortal159(::core::primitive::u8),
                        #[codec(index = 160)]
                        Mortal160(::core::primitive::u8),
                        #[codec(index = 161)]
                        Mortal161(::core::primitive::u8),
                        #[codec(index = 162)]
                        Mortal162(::core::primitive::u8),
                        #[codec(index = 163)]
                        Mortal163(::core::primitive::u8),
                        #[codec(index = 164)]
                        Mortal164(::core::primitive::u8),
                        #[codec(index = 165)]
                        Mortal165(::core::primitive::u8),
                        #[codec(index = 166)]
                        Mortal166(::core::primitive::u8),
                        #[codec(index = 167)]
                        Mortal167(::core::primitive::u8),
                        #[codec(index = 168)]
                        Mortal168(::core::primitive::u8),
                        #[codec(index = 169)]
                        Mortal169(::core::primitive::u8),
                        #[codec(index = 170)]
                        Mortal170(::core::primitive::u8),
                        #[codec(index = 171)]
                        Mortal171(::core::primitive::u8),
                        #[codec(index = 172)]
                        Mortal172(::core::primitive::u8),
                        #[codec(index = 173)]
                        Mortal173(::core::primitive::u8),
                        #[codec(index = 174)]
                        Mortal174(::core::primitive::u8),
                        #[codec(index = 175)]
                        Mortal175(::core::primitive::u8),
                        #[codec(index = 176)]
                        Mortal176(::core::primitive::u8),
                        #[codec(index = 177)]
                        Mortal177(::core::primitive::u8),
                        #[codec(index = 178)]
                        Mortal178(::core::primitive::u8),
                        #[codec(index = 179)]
                        Mortal179(::core::primitive::u8),
                        #[codec(index = 180)]
                        Mortal180(::core::primitive::u8),
                        #[codec(index = 181)]
                        Mortal181(::core::primitive::u8),
                        #[codec(index = 182)]
                        Mortal182(::core::primitive::u8),
                        #[codec(index = 183)]
                        Mortal183(::core::primitive::u8),
                        #[codec(index = 184)]
                        Mortal184(::core::primitive::u8),
                        #[codec(index = 185)]
                        Mortal185(::core::primitive::u8),
                        #[codec(index = 186)]
                        Mortal186(::core::primitive::u8),
                        #[codec(index = 187)]
                        Mortal187(::core::primitive::u8),
                        #[codec(index = 188)]
                        Mortal188(::core::primitive::u8),
                        #[codec(index = 189)]
                        Mortal189(::core::primitive::u8),
                        #[codec(index = 190)]
                        Mortal190(::core::primitive::u8),
                        #[codec(index = 191)]
                        Mortal191(::core::primitive::u8),
                        #[codec(index = 192)]
                        Mortal192(::core::primitive::u8),
                        #[codec(index = 193)]
                        Mortal193(::core::primitive::u8),
                        #[codec(index = 194)]
                        Mortal194(::core::primitive::u8),
                        #[codec(index = 195)]
                        Mortal195(::core::primitive::u8),
                        #[codec(index = 196)]
                        Mortal196(::core::primitive::u8),
                        #[codec(index = 197)]
                        Mortal197(::core::primitive::u8),
                        #[codec(index = 198)]
                        Mortal198(::core::primitive::u8),
                        #[codec(index = 199)]
                        Mortal199(::core::primitive::u8),
                        #[codec(index = 200)]
                        Mortal200(::core::primitive::u8),
                        #[codec(index = 201)]
                        Mortal201(::core::primitive::u8),
                        #[codec(index = 202)]
                        Mortal202(::core::primitive::u8),
                        #[codec(index = 203)]
                        Mortal203(::core::primitive::u8),
                        #[codec(index = 204)]
                        Mortal204(::core::primitive::u8),
                        #[codec(index = 205)]
                        Mortal205(::core::primitive::u8),
                        #[codec(index = 206)]
                        Mortal206(::core::primitive::u8),
                        #[codec(index = 207)]
                        Mortal207(::core::primitive::u8),
                        #[codec(index = 208)]
                        Mortal208(::core::primitive::u8),
                        #[codec(index = 209)]
                        Mortal209(::core::primitive::u8),
                        #[codec(index = 210)]
                        Mortal210(::core::primitive::u8),
                        #[codec(index = 211)]
                        Mortal211(::core::primitive::u8),
                        #[codec(index = 212)]
                        Mortal212(::core::primitive::u8),
                        #[codec(index = 213)]
                        Mortal213(::core::primitive::u8),
                        #[codec(index = 214)]
                        Mortal214(::core::primitive::u8),
                        #[codec(index = 215)]
                        Mortal215(::core::primitive::u8),
                        #[codec(index = 216)]
                        Mortal216(::core::primitive::u8),
                        #[codec(index = 217)]
                        Mortal217(::core::primitive::u8),
                        #[codec(index = 218)]
                        Mortal218(::core::primitive::u8),
                        #[codec(index = 219)]
                        Mortal219(::core::primitive::u8),
                        #[codec(index = 220)]
                        Mortal220(::core::primitive::u8),
                        #[codec(index = 221)]
                        Mortal221(::core::primitive::u8),
                        #[codec(index = 222)]
                        Mortal222(::core::primitive::u8),
                        #[codec(index = 223)]
                        Mortal223(::core::primitive::u8),
                        #[codec(index = 224)]
                        Mortal224(::core::primitive::u8),
                        #[codec(index = 225)]
                        Mortal225(::core::primitive::u8),
                        #[codec(index = 226)]
                        Mortal226(::core::primitive::u8),
                        #[codec(index = 227)]
                        Mortal227(::core::primitive::u8),
                        #[codec(index = 228)]
                        Mortal228(::core::primitive::u8),
                        #[codec(index = 229)]
                        Mortal229(::core::primitive::u8),
                        #[codec(index = 230)]
                        Mortal230(::core::primitive::u8),
                        #[codec(index = 231)]
                        Mortal231(::core::primitive::u8),
                        #[codec(index = 232)]
                        Mortal232(::core::primitive::u8),
                        #[codec(index = 233)]
                        Mortal233(::core::primitive::u8),
                        #[codec(index = 234)]
                        Mortal234(::core::primitive::u8),
                        #[codec(index = 235)]
                        Mortal235(::core::primitive::u8),
                        #[codec(index = 236)]
                        Mortal236(::core::primitive::u8),
                        #[codec(index = 237)]
                        Mortal237(::core::primitive::u8),
                        #[codec(index = 238)]
                        Mortal238(::core::primitive::u8),
                        #[codec(index = 239)]
                        Mortal239(::core::primitive::u8),
                        #[codec(index = 240)]
                        Mortal240(::core::primitive::u8),
                        #[codec(index = 241)]
                        Mortal241(::core::primitive::u8),
                        #[codec(index = 242)]
                        Mortal242(::core::primitive::u8),
                        #[codec(index = 243)]
                        Mortal243(::core::primitive::u8),
                        #[codec(index = 244)]
                        Mortal244(::core::primitive::u8),
                        #[codec(index = 245)]
                        Mortal245(::core::primitive::u8),
                        #[codec(index = 246)]
                        Mortal246(::core::primitive::u8),
                        #[codec(index = 247)]
                        Mortal247(::core::primitive::u8),
                        #[codec(index = 248)]
                        Mortal248(::core::primitive::u8),
                        #[codec(index = 249)]
                        Mortal249(::core::primitive::u8),
                        #[codec(index = 250)]
                        Mortal250(::core::primitive::u8),
                        #[codec(index = 251)]
                        Mortal251(::core::primitive::u8),
                        #[codec(index = 252)]
                        Mortal252(::core::primitive::u8),
                        #[codec(index = 253)]
                        Mortal253(::core::primitive::u8),
                        #[codec(index = 254)]
                        Mortal254(::core::primitive::u8),
                        #[codec(index = 255)]
                        Mortal255(::core::primitive::u8),
                    }
                }
                pub mod unchecked_extrinsic {
                    use super::runtime_types;
                    #[derive(
                        :: subxt :: ext :: codec :: Decode,
                        :: subxt :: ext :: codec :: Encode,
                        Clone,
                        Debug,
                        Eq,
                        PartialEq,
                    )]
                    pub struct UncheckedExtrinsic<_0, _1, _2, _3>(
                        pub ::std::vec::Vec<::core::primitive::u8>,
                        #[codec(skip)] pub ::core::marker::PhantomData<(_1, _0, _2, _3)>,
                    );
                }
            }
            pub mod multiaddress {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub enum MultiAddress<_0, _1> {
                    #[codec(index = 0)]
                    Id(_0),
                    #[codec(index = 1)]
                    Index(#[codec(compact)] _1),
                    #[codec(index = 2)]
                    Raw(::std::vec::Vec<::core::primitive::u8>),
                    #[codec(index = 3)]
                    Address32([::core::primitive::u8; 32usize]),
                    #[codec(index = 4)]
                    Address20([::core::primitive::u8; 20usize]),
                }
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum DispatchError {
                #[codec(index = 0)]
                Other,
                #[codec(index = 1)]
                CannotLookup,
                #[codec(index = 2)]
                BadOrigin,
                #[codec(index = 3)]
                Module(runtime_types::sp_runtime::ModuleError),
                #[codec(index = 4)]
                ConsumerRemaining,
                #[codec(index = 5)]
                NoProviders,
                #[codec(index = 6)]
                TooManyConsumers,
                #[codec(index = 7)]
                Token(runtime_types::sp_runtime::TokenError),
                #[codec(index = 8)]
                Arithmetic(runtime_types::sp_arithmetic::ArithmeticError),
                #[codec(index = 9)]
                Transactional(runtime_types::sp_runtime::TransactionalError),
                #[codec(index = 10)]
                Exhausted,
                #[codec(index = 11)]
                Corruption,
                #[codec(index = 12)]
                Unavailable,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct ModuleError {
                pub index: ::core::primitive::u8,
                pub error: [::core::primitive::u8; 4usize],
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum MultiSignature {
                #[codec(index = 0)]
                Ed25519(runtime_types::sp_core::ed25519::Signature),
                #[codec(index = 1)]
                Sr25519(runtime_types::sp_core::sr25519::Signature),
                #[codec(index = 2)]
                Ecdsa(runtime_types::sp_core::ecdsa::Signature),
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum TokenError {
                #[codec(index = 0)]
                NoFunds,
                #[codec(index = 1)]
                WouldDie,
                #[codec(index = 2)]
                BelowMinimum,
                #[codec(index = 3)]
                CannotCreate,
                #[codec(index = 4)]
                UnknownAsset,
                #[codec(index = 5)]
                Frozen,
                #[codec(index = 6)]
                Unsupported,
            }
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub enum TransactionalError {
                #[codec(index = 0)]
                LimitReached,
                #[codec(index = 1)]
                NoLayer,
            }
        }
        pub mod sp_version {
            use super::runtime_types;
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct RuntimeVersion {
                pub spec_name: ::std::string::String,
                pub impl_name: ::std::string::String,
                pub authoring_version: ::core::primitive::u32,
                pub spec_version: ::core::primitive::u32,
                pub impl_version: ::core::primitive::u32,
                pub apis:
                    ::std::vec::Vec<([::core::primitive::u8; 8usize], ::core::primitive::u32)>,
                pub transaction_version: ::core::primitive::u32,
                pub state_version: ::core::primitive::u8,
            }
        }
        pub mod sp_weights {
            use super::runtime_types;
            pub mod weight_v2 {
                use super::runtime_types;
                #[derive(
                    :: subxt :: ext :: codec :: Decode,
                    :: subxt :: ext :: codec :: Encode,
                    Clone,
                    Debug,
                    Eq,
                    PartialEq,
                )]
                pub struct Weight {
                    #[codec(compact)]
                    pub ref_time: ::core::primitive::u64,
                    #[codec(compact)]
                    pub proof_size: ::core::primitive::u64,
                }
            }
            #[derive(
                :: subxt :: ext :: codec :: CompactAs,
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct OldWeight(pub ::core::primitive::u64);
            #[derive(
                :: subxt :: ext :: codec :: Decode,
                :: subxt :: ext :: codec :: Encode,
                Clone,
                Debug,
                Eq,
                PartialEq,
            )]
            pub struct RuntimeDbWeight {
                pub read: ::core::primitive::u64,
                pub write: ::core::primitive::u64,
            }
        }
    }
    #[doc = r" The default error type returned when there is a runtime issue,"]
    #[doc = r" exposed here for ease of use."]
    pub type DispatchError = runtime_types::sp_runtime::DispatchError;
    pub fn constants() -> ConstantsApi {
        ConstantsApi
    }
    pub fn storage() -> StorageApi {
        StorageApi
    }
    pub fn tx() -> TransactionApi {
        TransactionApi
    }
    pub struct ConstantsApi;
    impl ConstantsApi {
        pub fn system(&self) -> system::constants::ConstantsApi {
            system::constants::ConstantsApi
        }
        pub fn scheduler(&self) -> scheduler::constants::ConstantsApi {
            scheduler::constants::ConstantsApi
        }
        pub fn timestamp(&self) -> timestamp::constants::ConstantsApi {
            timestamp::constants::ConstantsApi
        }
        pub fn balances(&self) -> balances::constants::ConstantsApi {
            balances::constants::ConstantsApi
        }
        pub fn transaction_payment(&self) -> transaction_payment::constants::ConstantsApi {
            transaction_payment::constants::ConstantsApi
        }
        pub fn staking(&self) -> staking::constants::ConstantsApi {
            staking::constants::ConstantsApi
        }
        pub fn elections(&self) -> elections::constants::ConstantsApi {
            elections::constants::ConstantsApi
        }
        pub fn treasury(&self) -> treasury::constants::ConstantsApi {
            treasury::constants::ConstantsApi
        }
        pub fn vesting(&self) -> vesting::constants::ConstantsApi {
            vesting::constants::ConstantsApi
        }
        pub fn utility(&self) -> utility::constants::ConstantsApi {
            utility::constants::ConstantsApi
        }
        pub fn multisig(&self) -> multisig::constants::ConstantsApi {
            multisig::constants::ConstantsApi
        }
        pub fn contracts(&self) -> contracts::constants::ConstantsApi {
            contracts::constants::ConstantsApi
        }
        pub fn nomination_pools(&self) -> nomination_pools::constants::ConstantsApi {
            nomination_pools::constants::ConstantsApi
        }
        pub fn identity(&self) -> identity::constants::ConstantsApi {
            identity::constants::ConstantsApi
        }
        pub fn baby_liminal(&self) -> baby_liminal::constants::ConstantsApi {
            baby_liminal::constants::ConstantsApi
        }
    }
    pub struct StorageApi;
    impl StorageApi {
        pub fn system(&self) -> system::storage::StorageApi {
            system::storage::StorageApi
        }
        pub fn randomness_collective_flip(
            &self,
        ) -> randomness_collective_flip::storage::StorageApi {
            randomness_collective_flip::storage::StorageApi
        }
        pub fn scheduler(&self) -> scheduler::storage::StorageApi {
            scheduler::storage::StorageApi
        }
        pub fn aura(&self) -> aura::storage::StorageApi {
            aura::storage::StorageApi
        }
        pub fn timestamp(&self) -> timestamp::storage::StorageApi {
            timestamp::storage::StorageApi
        }
        pub fn balances(&self) -> balances::storage::StorageApi {
            balances::storage::StorageApi
        }
        pub fn transaction_payment(&self) -> transaction_payment::storage::StorageApi {
            transaction_payment::storage::StorageApi
        }
        pub fn authorship(&self) -> authorship::storage::StorageApi {
            authorship::storage::StorageApi
        }
        pub fn staking(&self) -> staking::storage::StorageApi {
            staking::storage::StorageApi
        }
        pub fn history(&self) -> history::storage::StorageApi {
            history::storage::StorageApi
        }
        pub fn session(&self) -> session::storage::StorageApi {
            session::storage::StorageApi
        }
        pub fn aleph(&self) -> aleph::storage::StorageApi {
            aleph::storage::StorageApi
        }
        pub fn elections(&self) -> elections::storage::StorageApi {
            elections::storage::StorageApi
        }
        pub fn treasury(&self) -> treasury::storage::StorageApi {
            treasury::storage::StorageApi
        }
        pub fn vesting(&self) -> vesting::storage::StorageApi {
            vesting::storage::StorageApi
        }
        pub fn multisig(&self) -> multisig::storage::StorageApi {
            multisig::storage::StorageApi
        }
        pub fn sudo(&self) -> sudo::storage::StorageApi {
            sudo::storage::StorageApi
        }
        pub fn contracts(&self) -> contracts::storage::StorageApi {
            contracts::storage::StorageApi
        }
        pub fn nomination_pools(&self) -> nomination_pools::storage::StorageApi {
            nomination_pools::storage::StorageApi
        }
        pub fn identity(&self) -> identity::storage::StorageApi {
            identity::storage::StorageApi
        }
        pub fn baby_liminal(&self) -> baby_liminal::storage::StorageApi {
            baby_liminal::storage::StorageApi
        }
    }
    pub struct TransactionApi;
    impl TransactionApi {
        pub fn system(&self) -> system::calls::TransactionApi {
            system::calls::TransactionApi
        }
        pub fn scheduler(&self) -> scheduler::calls::TransactionApi {
            scheduler::calls::TransactionApi
        }
        pub fn timestamp(&self) -> timestamp::calls::TransactionApi {
            timestamp::calls::TransactionApi
        }
        pub fn balances(&self) -> balances::calls::TransactionApi {
            balances::calls::TransactionApi
        }
        pub fn staking(&self) -> staking::calls::TransactionApi {
            staking::calls::TransactionApi
        }
        pub fn session(&self) -> session::calls::TransactionApi {
            session::calls::TransactionApi
        }
        pub fn aleph(&self) -> aleph::calls::TransactionApi {
            aleph::calls::TransactionApi
        }
        pub fn elections(&self) -> elections::calls::TransactionApi {
            elections::calls::TransactionApi
        }
        pub fn treasury(&self) -> treasury::calls::TransactionApi {
            treasury::calls::TransactionApi
        }
        pub fn vesting(&self) -> vesting::calls::TransactionApi {
            vesting::calls::TransactionApi
        }
        pub fn utility(&self) -> utility::calls::TransactionApi {
            utility::calls::TransactionApi
        }
        pub fn multisig(&self) -> multisig::calls::TransactionApi {
            multisig::calls::TransactionApi
        }
        pub fn sudo(&self) -> sudo::calls::TransactionApi {
            sudo::calls::TransactionApi
        }
        pub fn contracts(&self) -> contracts::calls::TransactionApi {
            contracts::calls::TransactionApi
        }
        pub fn nomination_pools(&self) -> nomination_pools::calls::TransactionApi {
            nomination_pools::calls::TransactionApi
        }
        pub fn identity(&self) -> identity::calls::TransactionApi {
            identity::calls::TransactionApi
        }
        pub fn baby_liminal(&self) -> baby_liminal::calls::TransactionApi {
            baby_liminal::calls::TransactionApi
        }
    }
    #[doc = r" check whether the Client you are using is aligned with the statically generated codegen."]
    pub fn validate_codegen<T: ::subxt::Config, C: ::subxt::client::OfflineClientT<T>>(
        client: &C,
    ) -> Result<(), ::subxt::error::MetadataError> {
        let runtime_metadata_hash = client.metadata().metadata_hash(&PALLETS);
        if runtime_metadata_hash
            != [
                59u8, 70u8, 133u8, 112u8, 201u8, 154u8, 103u8, 142u8, 36u8, 177u8, 196u8, 58u8,
                112u8, 214u8, 78u8, 95u8, 13u8, 3u8, 11u8, 93u8, 154u8, 28u8, 165u8, 131u8, 232u8,
                79u8, 46u8, 55u8, 245u8, 136u8, 7u8, 114u8,
            ]
        {
            Err(::subxt::error::MetadataError::IncompatibleMetadata)
        } else {
            Ok(())
        }
    }
}
