use ink_env::{
    call::{build_call, Call, ExecutionInput, Selector},
    AccountId, DefaultEnvironment, Error as InkEnvError,
};

use crate::access_control::{Role, HAS_ROLE_SELECTOR};

pub trait AccessControlled {
    type ContractError;

    fn check_role<ContractError>(
        access_control: AccountId,
        account: AccountId,
        role: Role,
        contract_call_error_handler: fn(why: InkEnvError) -> ContractError,
        access_control_error_handler: fn() -> ContractError,
    ) -> Result<(), ContractError> {
        match build_call::<DefaultEnvironment>()
            .call_type(Call::new().callee(access_control))
            .exec_input(
                ExecutionInput::new(Selector::new(HAS_ROLE_SELECTOR))
                    .push_arg(account)
                    .push_arg(role),
            )
            .returns::<bool>()
            .fire()
        {
            Ok(has_role) => match has_role {
                true => Ok(()),
                false => Err(access_control_error_handler()),
            },
            Err(why) => Err(contract_call_error_handler(why)),
        }
    }
}
