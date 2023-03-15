# TheButton

Series of smart contract based games, with varying tokenomics.
TheButton is loosely based on the famous [game](https://en.wikipedia.org/wiki/The_Button_(Reddit)) on reddit.

- Button lives for a set time.
- Pressing the button extends its life.
- Users are rewarded for playing the game.
- Everybody can play only once.

```
  |______|_______|
  |      |       |
start   now    deadline
```

## EarlyBirdSpecial

There is a pre-minted amount of tokens (a classic ERC20 standard).
Users are rewarded for pressing as early on as possible:

```
score = deadline - now
```

There are two built-in incentives:
* playing for the score: If you clicked in the 10th second of TheButton's life, which is set for example to 900 blocks, you get rewarded based on the score of 900-10=890 (and the button's life now will end at block 910).
* playing to be ThePressiah: the last player to click gets 20% of the total reward pool (1/4 of the sum of all the rewards paid for pressing the button)

## BackToTheFuture

In this scenario the rewards are reversed - players get rewarded for extending the button's life further into the future, i.e.:

```
score = now - start
```

The Pressiah gets 20% of the total reward pool.

## ThePressiahCometh

Game continues in perpetuity (but in practice as long as there are accounts that can still play it)

- In each iteration of the game TheButton lives for a number of blocks
- Clicking TheButton resets its countdown timer (extending the button's life further into the future)
- Tokens are continuously minted at the end of each iteration
- Players are rewarded for playing, with the ultimate goal of being the Pressiah (the last person to click the button)
- Reward rules:
  - If you're not ThePressiah, you get _k_ tokens if you pressed the button as the _k-th_ person in a row.
  - ThePressiah gets 20% of the total reward pool.

# Development

## Prerequisites

- Rust nightly
- cargo-contract compatible with current node: `cargo install cargo-contract --version 2.0.1`

## Instructions

Firstly bootstrap a one-node  `smartnet` chain:

```bash
 ./.github/scripts/run_smartnet.sh
```

Secondly `deploy` script takes care of compiling and deploying the contracts.

```bash
source contracts/env/dev && ./contracts/scripts/deploy.sh
```

Specifically it will:

- Deploy the contracts.
- Set access control on them.
- Make necessary token transfers.

Third `test.sh` script plays the game from two well-known dev addresses.

```bash
./contracts/scripts/test.sh
```

It will:

- Interact with the games from known accounts.
- Wait past the game deadline, trigger the game end and reward distribution.
