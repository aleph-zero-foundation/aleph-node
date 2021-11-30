use substrate_api_client::Balance;

use crate::{Connection, TransferTransaction};

#[derive(Debug)]
pub struct FeeInfo {
    pub fee_without_weight: Balance,
    pub unadjusted_weight: Balance,
    pub adjusted_weight: Balance,
}

pub fn get_tx_fee_info(connection: &Connection, tx: &TransferTransaction) -> FeeInfo {
    let unadjusted_weight = connection
        .get_payment_info(&tx.hex_encode(), None)
        .unwrap()
        .unwrap()
        .weight as Balance;

    let fee = connection
        .get_fee_details(&tx.hex_encode(), None)
        .unwrap()
        .unwrap();
    let inclusion_fee = fee.inclusion_fee.unwrap();

    FeeInfo {
        fee_without_weight: inclusion_fee.base_fee + inclusion_fee.len_fee + fee.tip,
        unadjusted_weight,
        adjusted_weight: inclusion_fee.adjusted_weight_fee,
    }
}
