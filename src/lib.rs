#![cfg_attr(not(any(feature = "export-abi", test)), no_main)]
extern crate alloc;

use alloc::vec::Vec;
use alloy_primitives::{Address, FixedBytes, U256};
use alloy_sol_types::sol;
use stylus_sdk::{
    abi::Bytes,
    function_selector,
    host::VM,
    prelude::*,
    storage::{StorageBool, StorageMap},
};

// ERC-7201 storage slot for "token.transferable"
// Calculated as: keccak256(abi.encode(uint256(keccak256("token.transferable")) - 1)) & ~bytes32(uint256(0xff))
const TRANSFERABLE_STORAGE_POSITION: U256 = U256::from_be_bytes([
    0x32, 0x4c, 0x74, 0xba, 0x20, 0x97, 0x60, 0x24, 0x4d, 0x63, 0x14, 0x3f, 0xd3, 0x3a, 0xf4, 0x2e,
    0x11, 0x68, 0x1d, 0x6b, 0x19, 0x8f, 0x7e, 0x7e, 0x1b, 0x6e, 0xf3, 0xeb, 0x45, 0x97, 0x6d, 0x00
]);

sol! {
    #[derive(Debug, AbiType)]
    struct CallbackFunction {
        bytes4 selector;
    }

    #[derive(Debug, AbiType)]
    struct FallbackFunction {
        bytes4 selector;
        uint256 permissionBits;
    }

    #[derive(Debug, AbiType)]
    struct ModuleConfig {
        bool registerInstallationCallback;
        bytes4[] requiredInterfaces;
        bytes4[] supportedInterfaces;
        CallbackFunction[] callbackFunctions;
        FallbackFunction[] fallbackFunctions;
    }
}

struct TransferableStorage {
    transfer_enabled: StorageBool,
    transfer_enabled_for: StorageMap<Address, StorageBool>,
}

impl TransferableStorage {
    fn load(vm: &VM) -> Self {
        unsafe {
            Self {
                transfer_enabled: StorageBool::new(TRANSFERABLE_STORAGE_POSITION, 0, vm.clone()),
                transfer_enabled_for: StorageMap::new(TRANSFERABLE_STORAGE_POSITION + U256::from(1), 0, vm.clone()),
            }
        }
    }
}

sol_storage! {
    #[entrypoint]
    pub struct StylusTransferableERC1155 {
    }
}

#[public]
impl StylusTransferableERC1155 {
    #[constructor]
    pub fn constructor(&mut self) -> Result<(), String> {
        Ok(())
    }

    pub fn get_module_config(&self) -> Result<ModuleConfig, Vec<u8>> {
        Ok(ModuleConfig {
            registerInstallationCallback: false,
            requiredInterfaces: vec![
                FixedBytes::from([0xd9, 0xb6, 0x7a, 0x26]), // ERC1155 interface
            ],
            supportedInterfaces: vec![],
            callbackFunctions: vec![
                CallbackFunction {
                    selector: FixedBytes::from(function_selector!("beforeTransferERC1155", Address, Address, U256, U256)),
                },
                CallbackFunction {
                    selector: FixedBytes::from(function_selector!("beforeBatchTransferERC1155", Address, Address, Vec<U256>, Vec<U256>)),
                },
            ],
            fallbackFunctions: vec![
                FallbackFunction {
                    selector: FixedBytes::from(function_selector!("isTransferEnabled")),
                    permissionBits: U256::ZERO,
                },
                FallbackFunction {
                    selector: FixedBytes::from(function_selector!("isTransferEnabledFor", Address)),
                    permissionBits: U256::ZERO,
                },
                FallbackFunction {
                    selector: FixedBytes::from(function_selector!("setTransferable", bool)),
                    permissionBits: U256::from(2), // _MANAGER_ROLE
                },
                FallbackFunction {
                    selector: FixedBytes::from(function_selector!("setTransferableFor", Address, bool)),
                    permissionBits: U256::from(2), // _MANAGER_ROLE
                },
            ],
        })
    }

    #[selector(name = "beforeTransferERC1155")]
    pub fn before_transfer_erc1155(
        &mut self,
        from: Address,
        to: Address,
        _id: U256,
        _amount: U256
    ) -> Result<Bytes, String> {
        let storage = TransferableStorage::load(&self.vm());
        
        let is_operator_allowed = 
            storage.transfer_enabled_for.get(self.vm().msg_sender()) ||
            storage.transfer_enabled_for.get(from) ||
            storage.transfer_enabled_for.get(to);

        if !is_operator_allowed && !storage.transfer_enabled.get() {
            return Err("Transfer disabled".into());
        }
        
        Ok(Bytes(vec![].into()))
    }

    #[selector(name = "beforeBatchTransferERC1155")]
    pub fn before_batch_transfer_erc1155(
        &mut self,
        from: Address,
        to: Address,
        _ids: Vec<U256>,
        _amounts: Vec<U256>
    ) -> Result<Bytes, String> {
        let storage = TransferableStorage::load(&self.vm());
        
        let is_operator_allowed = 
            storage.transfer_enabled_for.get(self.vm().msg_sender()) ||
            storage.transfer_enabled_for.get(from) ||
            storage.transfer_enabled_for.get(to);

        if !is_operator_allowed && !storage.transfer_enabled.get() {
            return Err("Transfer disabled".into());
        }
        
        Ok(Bytes(vec![].into()))
    }

    pub fn is_transfer_enabled(&self) -> bool {
        TransferableStorage::load(&self.vm()).transfer_enabled.get()
    }

    pub fn is_transfer_enabled_for(&self, target: Address) -> bool {
        TransferableStorage::load(&self.vm()).transfer_enabled_for.get(target)
    }

    pub fn set_transferable(&mut self, enable_transfer: bool) -> Result<(), String> {
        TransferableStorage::load(&self.vm()).transfer_enabled.set(enable_transfer);
        Ok(())
    }

    pub fn set_transferable_for(&mut self, target: Address, enable_transfer: bool) -> Result<(), String> {
        TransferableStorage::load(&self.vm()).transfer_enabled_for.insert(target, enable_transfer);
        Ok(())
    }
}