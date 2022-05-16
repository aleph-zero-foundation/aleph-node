use crate::AnyConnection;
use codec::Encode;
use substrate_api_client::{Balance, UncheckedExtrinsicV4};

#[derive(Debug)]
pub struct FeeInfo {
    pub fee_without_weight: Balance,
    pub unadjusted_weight: Balance,
    pub adjusted_weight: Balance,
}

pub fn get_tx_fee_info<C: AnyConnection, Call: Encode>(
    connection: &C,
    tx: &UncheckedExtrinsicV4<Call>,
) -> FeeInfo {
    let unadjusted_weight = connection
        .as_connection()
        .get_payment_info(&tx.hex_encode(), None)
        .expect("Should access payment info")
        .expect("Payment info should be present")
        .weight as Balance;

    let fee = connection
        .as_connection()
        .get_fee_details(&tx.hex_encode(), None)
        .expect("Should access fee details")
        .expect("Should read fee details");
    let inclusion_fee = fee.inclusion_fee.expect("Transaction should be payable");

    FeeInfo {
        fee_without_weight: inclusion_fee.base_fee + inclusion_fee.len_fee + fee.tip,
        unadjusted_weight,
        adjusted_weight: inclusion_fee.adjusted_weight_fee,
    }
}

pub fn get_next_fee_multiplier<C: AnyConnection>(connection: &C) -> u128 {
    connection
        .as_connection()
        .get_storage_value("TransactionPayment", "NextFeeMultiplier", None)
        .expect("Should access storage")
        .expect("Key `NextFeeMultiplier` should be present in storage")
}
