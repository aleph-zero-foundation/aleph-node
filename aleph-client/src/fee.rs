use substrate_api_client::Balance;

use crate::{AnyConnection, BalanceTransfer, FeeInfo, GetTxInfo, ReadStorage, SignedConnection};

impl GetTxInfo<<SignedConnection as BalanceTransfer>::TransferTx> for SignedConnection {
    fn get_tx_info(&self, tx: &<SignedConnection as BalanceTransfer>::TransferTx) -> FeeInfo {
        let tx = self.create_transfer_extrinsic(tx.clone());
        let tx_hex = tx.hex_encode();
        let unadjusted_weight = self
            .as_connection()
            .get_payment_info(&tx_hex, None)
            .expect("Should access payment info")
            .expect("Payment info should be present")
            .weight as Balance;

        let fee = self
            .as_connection()
            .get_fee_details(&tx_hex, None)
            .expect("Should access fee details")
            .expect("Should read fee details");
        let inclusion_fee = fee.inclusion_fee.expect("Transaction should be payable");

        FeeInfo {
            fee_without_weight: inclusion_fee.base_fee + inclusion_fee.len_fee + fee.tip,
            unadjusted_weight,
            adjusted_weight: inclusion_fee.adjusted_weight_fee,
        }
    }
}

pub fn get_next_fee_multiplier<C: ReadStorage>(connection: &C) -> u128 {
    connection.read_storage_value("TransactionPayment", "NextFeeMultiplier")
}
