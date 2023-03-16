use std::time::Duration;

use aleph_client::{contract_transcode::Value, pallets::system::SystemApi};
use anyhow::Result;
use assert2::{assert, let_assert};
use helpers::sign;
use log::info;
use serial_test::serial;
use tokio::time::sleep;

use crate::{
    config::{setup_test, Config},
    test::button_game::helpers::{
        alephs, assert_recv_id, mega, refute_recv_id, setup_button_test, setup_dex_test,
        setup_wrapped_azero_test, wait_for_death, ButtonTestContext, DexTestContext,
        WAzeroTestContext,
    },
};

mod contracts;
mod helpers;

/// Test wrapped azero
///
/// The scenario:
///
/// 1. Wraps some azero and checks that the PSP22 balance increased accordingly.
/// 2. Unwraps half of the amount, checks that some wrapped funds remained while the rest has been returned to azero,
///    minus fees.
#[tokio::test]
#[serial]
pub async fn wrapped_azero() -> Result<()> {
    let config = setup_test();
    let WAzeroTestContext {
        conn,
        account,
        wazero,
        mut events,
        ..
    } = setup_wrapped_azero_test(config).await?;
    let account_conn = &sign(&conn, &account);
    let account_id = account.account_id();

    wazero.wrap(account_conn, alephs(2)).await?;

    let event = assert_recv_id(&mut events, "Wrapped").await;
    let_assert!(Some(Value::Literal(acc_id)) = event.data.get("caller"));
    assert!(*acc_id == account_id.to_string());
    assert!(event.data.get("amount") == Some(&Value::UInt(alephs(2))));
    assert!(
        wazero
            .balance_of(account_conn, account.account_id())
            .await?
            == alephs(2)
    );

    let balance_before = conn.get_free_balance(account_id.clone(), None).await;
    wazero.unwrap(account_conn, alephs(1)).await?;

    let event = assert_recv_id(&mut events, "UnWrapped").await;
    let balance_after = conn.get_free_balance(account_id.clone(), None).await;
    let max_fee = alephs(1) / 100;
    assert!(balance_after - balance_before > alephs(1) - max_fee);
    let_assert!(Some(Value::Literal(acc_id)) = event.data.get("caller"));
    assert!(*acc_id == account_id.to_string());
    assert!(event.data.get("amount") == Some(&Value::UInt(alephs(1))));
    assert!(
        wazero
            .balance_of(account_conn, account.account_id())
            .await?
            == alephs(1)
    );

    Ok(())
}

/// Test trading on simple_dex.
///
/// The scenario does the following (given 3 tokens A, B, C):
///
/// 1. Enables A <-> B, and A -> C swaps.
/// 2. Adds (A, 2000M), (B, 5000M), (C, 10000M) of liquidity.
/// 3. Makes a swap A -> B and then B -> A for the amount of B received in the first swap.
/// 4. Makes a swap A -> B expecting negative slippage (this should fail).
/// 5. Checks that the price after the two swaps is the same as before (with a dust allowance of 1 for rounding).
/// 6. Checks that it's possible to make an A -> C swap, but impossible to make a C -> A swap.
#[tokio::test]
#[serial]
pub async fn simple_dex() -> Result<()> {
    let config = setup_test();
    let DexTestContext {
        conn,
        authority,
        account,
        dex,
        token1,
        token2,
        token3,
        mut events,
    } = setup_dex_test(config).await?;

    let authority_conn = &sign(&conn, &authority);
    let account_conn = &sign(&conn, &account);
    let token1 = token1.as_ref();
    let token2 = token2.as_ref();
    let token3 = token3.as_ref();
    let dex = dex.as_ref();

    dex.add_swap_pair(authority_conn, token1.into(), token2.into())
        .await?;
    assert_recv_id(&mut events, "SwapPairAdded").await;

    dex.add_swap_pair(authority_conn, token2.into(), token1.into())
        .await?;
    assert_recv_id(&mut events, "SwapPairAdded").await;

    dex.add_swap_pair(authority_conn, token1.into(), token3.into())
        .await?;
    assert_recv_id(&mut events, "SwapPairAdded").await;

    token1
        .mint(authority_conn, authority.account_id(), mega(3000))
        .await?;
    assert_recv_id(&mut events, "Transfer").await;
    token2
        .mint(authority_conn, authority.account_id(), mega(5000))
        .await?;
    assert_recv_id(&mut events, "Transfer").await;
    token3
        .mint(authority_conn, authority.account_id(), mega(10000))
        .await?;
    assert_recv_id(&mut events, "Transfer").await;

    token1
        .approve(authority_conn, &dex.into(), mega(3000))
        .await?;
    assert_recv_id(&mut events, "Approval").await;
    token2
        .approve(authority_conn, &dex.into(), mega(5000))
        .await?;
    assert_recv_id(&mut events, "Approval").await;
    token3
        .approve(authority_conn, &dex.into(), mega(10000))
        .await?;
    assert_recv_id(&mut events, "Approval").await;

    dex.deposit(
        authority_conn,
        &[
            (token1, mega(3000)),
            (token2, mega(5000)),
            (token3, mega(10000)),
        ],
    )
    .await?;
    assert_recv_id(&mut events, "Deposited").await;

    let more_than_liquidity = mega(1_000_000);
    let res = dex
        .swap(account_conn, token1, 100, token2, more_than_liquidity)
        .await;
    assert!(res.is_err());
    refute_recv_id(&mut events, "Swapped").await;

    let initial_amount = mega(100);
    token1
        .mint(authority_conn, account.account_id(), initial_amount)
        .await?;
    let expected_output = dex
        .out_given_in(account_conn, token1, token2, initial_amount)
        .await?;
    assert!(expected_output > 0);

    let at_most_10_percent_slippage = expected_output * 9 / 10;
    token1
        .approve(account_conn, &dex.into(), initial_amount)
        .await?;
    dex.swap(
        account_conn,
        token1,
        initial_amount,
        token2,
        at_most_10_percent_slippage,
    )
    .await?;
    assert_recv_id(&mut events, "Swapped").await;
    assert!(token2.balance_of(&conn, account.account_id()).await? == expected_output);

    token2
        .approve(account_conn, &dex.into(), expected_output)
        .await?;
    dex.swap(account_conn, token2, expected_output, token1, mega(90))
        .await?;
    assert_recv_id(&mut events, "Swapped").await;

    let balance_after = token1.balance_of(&conn, account.account_id()).await?;
    assert!(initial_amount.abs_diff(balance_after) <= 1);
    assert!(
        dex.out_given_in(account_conn, token1, token2, initial_amount)
            .await?
            .abs_diff(expected_output)
            <= 1
    );

    token1
        .approve(account_conn, &dex.into(), balance_after)
        .await?;
    let unreasonable_slippage = expected_output * 11 / 10;
    assert!(
        dex.swap(
            account_conn,
            token1,
            balance_after,
            token2,
            unreasonable_slippage,
        )
        .await
        .is_err(),
        "expected swap to fail"
    );
    refute_recv_id(&mut events, "Swapped").await;

    dex.swap(account_conn, token1, balance_after, token3, mega(90))
        .await?;
    assert_recv_id(&mut events, "Swapped").await;

    // can't swap a pair not on the whitelist
    dex.remove_swap_pair(authority_conn, token3.into(), token1.into())
        .await?;
    assert_recv_id(&mut events, "SwapPairRemoved").await;

    let balance_token3 = token3.balance_of(&conn, account.account_id()).await?;
    token3
        .approve(account_conn, &dex.into(), balance_token3)
        .await?;
    assert!(
        dex.swap(account_conn, token3, balance_token3, token1, mega(90))
            .await
            .is_err(),
        "can't swap pair that is not whitelisted"
    );
    refute_recv_id(&mut events, "Swapped").await;

    Ok(())
}

/// Tests trading on the marketplace.
///
/// The scenario:
///
/// 1. Buys a ticket without setting the max price (this should succeed).
/// 2. Tries to buy a ticket with setting the max price too low (this should fail).
/// 3. Tries to buy a ticket with setting the max price appropriately (this should succeed).
#[tokio::test]
#[serial]
pub async fn marketplace() -> Result<()> {
    let config = setup_test();
    let ButtonTestContext {
        conn,
        authority,
        player,
        marketplace,
        ticket_token,
        reward_token,
        mut events,
        ..
    } = setup_button_test(config, &config.test_case_params.early_bird_special).await?;
    let player = &player;

    marketplace.reset(&sign(&conn, &authority)).await?;
    assert_recv_id(&mut events, "Reset").await;
    ticket_token
        .transfer(&sign(&conn, &authority), &marketplace.as_ref().into(), 2)
        .await?;

    let early_price = marketplace.price(&conn).await?;
    sleep(Duration::from_secs(2)).await;
    let later_price = marketplace.price(&conn).await?;
    assert!(later_price < early_price);

    let player_balance = 100 * later_price;
    reward_token
        .mint(
            &sign(&conn, &authority),
            player.account_id(),
            player_balance,
        )
        .await?;
    reward_token
        .approve(
            &sign(&conn, player),
            &marketplace.as_ref().into(),
            later_price,
        )
        .await?;
    marketplace.buy(&sign(&conn, player), None).await?;

    let event = assert_recv_id(&mut events, "Bought").await;
    assert!(event.contract == marketplace.as_ref().into());
    let_assert!(Some(&Value::UInt(price)) = event.data.get("price"));
    assert!(price <= later_price);
    let_assert!(Some(Value::Literal(acc_id)) = event.data.get("account_id"));
    assert!(acc_id == &player.account_id().to_string());
    assert!(ticket_token.balance_of(&conn, player.account_id()).await? == 1);
    assert!(reward_token.balance_of(&conn, player.account_id()).await? <= player_balance - price);
    assert!(marketplace.price(&conn).await? > price);

    let latest_price = marketplace.price(&conn).await?;

    info!("Setting max price too low");
    assert!(
        marketplace
            .buy(&sign(&conn, player), Some(latest_price / 2))
            .await
            .is_err(),
        "set price too low, should fail"
    );
    refute_recv_id(&mut events, "Bought").await;
    assert!(ticket_token.balance_of(&conn, player.account_id()).await? == 1);

    info!("Setting max price high enough");
    marketplace
        .buy(&sign(&conn, player), Some(latest_price * 2))
        .await?;
    assert_recv_id(&mut events, "Bought").await;
    assert!(ticket_token.balance_of(&conn, player.account_id()).await? == 2);

    Ok(())
}

/// Tests resetting the button game.
#[tokio::test]
#[serial]
pub async fn button_game_reset() -> Result<()> {
    let config = setup_test();
    let ButtonTestContext {
        conn,
        button,
        mut events,
        authority,
        marketplace,
        ticket_token,
        ..
    } = setup_button_test(config, &config.test_case_params.early_bird_special).await?;

    let deadline_old = button.deadline(&conn).await?;
    let marketplace_initial = ticket_token
        .balance_of(&conn, &marketplace.as_ref().into())
        .await?;
    ticket_token
        .transfer(&sign(&conn, &authority), &button.as_ref().into(), 1)
        .await?;

    wait_for_death(&conn, &button).await?;
    button.reset(&sign(&conn, &authority)).await?;

    assert_recv_id(&mut events, "GameReset").await;
    assert_recv_id(&mut events, "Reset").await;

    let deadline_new = button.deadline(&conn).await?;
    assert!(deadline_new > deadline_old);
    assert!(
        ticket_token
            .balance_of(&conn, &marketplace.as_ref().into())
            .await?
            == marketplace_initial + 1
    );

    Ok(())
}

#[tokio::test]
#[serial]
pub async fn early_bird_special() -> Result<()> {
    let config = setup_test();
    button_game_play(
        config,
        &config.test_case_params.early_bird_special,
        |early_presser_score, late_presser_score| {
            assert!(early_presser_score > late_presser_score);
        },
    )
    .await
}

#[tokio::test]
#[serial]
pub async fn back_to_the_future() -> Result<()> {
    let config = setup_test();
    button_game_play(
        config,
        &config.test_case_params.back_to_the_future,
        |early_presser_score, late_presser_score| {
            assert!(early_presser_score < late_presser_score);
        },
    )
    .await
}

#[tokio::test]
#[serial]
pub async fn the_pressiah_cometh() -> Result<()> {
    let config = setup_test();
    button_game_play(
        config,
        &config.test_case_params.the_pressiah_cometh,
        |early_presser_score, late_presser_score| {
            assert!(early_presser_score == 1);
            assert!(late_presser_score == 2);
        },
    )
    .await
}

/// Tests a basic scenario of playing the game.
///
/// The scenario:
///
/// 1. Resets the button.
/// 2. Gives 2 tickets to the player.
/// 3. Presses the button.
/// 4. Waits a bit and presses the button again.
/// 5. Waits until the button dies and checks the pressiah's score.
///
/// Passes the scores received by an early presser and late presser to `score_check` so that different scoring rules
/// can be tested generically.
async fn button_game_play<F: Fn(u128, u128)>(
    config: &Config,
    button_contract_address: &Option<String>,
    score_check: F,
) -> Result<()> {
    let ButtonTestContext {
        conn,
        button,
        mut events,
        authority,
        ticket_token,
        reward_token,
        player,
        ..
    } = setup_button_test(config, button_contract_address).await?;
    let player = &player;

    ticket_token
        .transfer(&sign(&conn, &authority), player.account_id(), 2)
        .await?;
    wait_for_death(&conn, &button).await?;
    button.reset(&sign(&conn, &authority)).await?;
    let event = assert_recv_id(&mut events, "GameReset").await;
    let_assert!(Some(&Value::UInt(reset_at)) = event.data.get("when"));
    let old_button_balance = ticket_token
        .balance_of(&conn, &button.as_ref().into())
        .await?;

    ticket_token
        .approve(&sign(&conn, player), &button.as_ref().into(), 2)
        .await?;
    button.press(&sign(&conn, player)).await?;

    let event = assert_recv_id(&mut events, "ButtonPressed").await;
    let_assert!(Some(&Value::UInt(first_presser_score)) = event.data.get("score"));
    assert!(event.data.get("by") == Some(&Value::Literal(player.account_id().to_string())));
    assert!(reward_token.balance_of(&conn, player.account_id()).await? == first_presser_score);
    assert!(first_presser_score > 0);
    assert!(ticket_token.balance_of(&conn, player.account_id()).await? == 1);
    assert!(
        ticket_token
            .balance_of(&conn, &button.as_ref().into())
            .await?
            == old_button_balance + 1
    );
    let_assert!(Some(&Value::UInt(first_press_at)) = event.data.get("when"));

    info!("Waiting before pressing again");
    sleep(Duration::from_secs(3)).await;

    button.press(&sign(&conn, player)).await?;
    let event = assert_recv_id(&mut events, "ButtonPressed").await;
    let_assert!(Some(&Value::UInt(second_presser_score)) = event.data.get("score"));
    let_assert!(Some(&Value::UInt(second_press_at)) = event.data.get("when"));
    let (early_presser_score, late_presser_score) =
        if (first_press_at - reset_at) < (second_press_at - first_press_at) {
            (first_presser_score, second_presser_score)
        } else {
            (second_presser_score, first_presser_score)
        };

    score_check(early_presser_score, late_presser_score);
    let total_score = early_presser_score + late_presser_score;
    assert!(reward_token.balance_of(&conn, player.account_id()).await? == total_score);

    wait_for_death(&conn, &button).await?;
    button.reset(&sign(&conn, &authority)).await?;
    assert_recv_id(&mut events, "Reset").await;

    let pressiah_score = total_score / 4;
    assert!(
        reward_token.balance_of(&conn, player.account_id()).await? == total_score + pressiah_score
    );

    Ok(())
}
