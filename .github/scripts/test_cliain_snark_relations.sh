#!/usr/bin/env bash

set -euo pipefail

CLIAIN=./bin/cliain/target/release/cliain

echo "Testing 'xor' relation"
${CLIAIN} snark-relation generate-keys xor
${CLIAIN} snark-relation generate-proof -p xor.groth16.pk.bytes xor -a 10 -b 11 -c 1
${CLIAIN} snark-relation verify \
  --verifying-key-file xor.groth16.vk.bytes \
  --proof-file xor.groth16.proof.bytes \
  --public-input-file xor.groth16.public_input.bytes

echo "Testing 'linear equation' relation"
${CLIAIN} snark-relation generate-keys linear-equation
${CLIAIN} snark-relation generate-proof -p linear_equation.groth16.pk.bytes linear-equation
${CLIAIN} snark-relation verify \
  --verifying-key-file linear_equation.groth16.vk.bytes \
  --proof-file linear_equation.groth16.proof.bytes \
  --public-input-file linear_equation.groth16.public_input.bytes

echo "Testing 'deposit' relation"
${CLIAIN} snark-relation generate-keys deposit
${CLIAIN} snark-relation generate-proof -p deposit.groth16.pk.bytes deposit \
  --note "2257517311045912551,9329547706917600007,17678219388335595033,2758194574870438734" \
  --token-id 1 \
  --token-amount 10 \
  --trapdoor 17 \
  --nullifier 19
${CLIAIN} snark-relation verify \
  --verifying-key-file deposit.groth16.vk.bytes \
  --proof-file deposit.groth16.proof.bytes \
  --public-input-file deposit.groth16.public_input.bytes

echo "Testing 'deposit-and-merge' relation"
${CLIAIN} snark-relation generate-keys deposit-and-merge
${CLIAIN} snark-relation generate-proof -p deposit_and_merge.groth16.pk.bytes deposit-and-merge \
  --max-path-len 4 \
  --token-id 1 \
  --old-nullifier 19 \
  --new-note "16873374675910556188,7520532372021931293,2252601191743571102,1137408349476470620" \
  --token-amount 3 \
  --merkle-root "13246676254120042674,6753642814231402535,5530105312079941569,7478542936335342845" \
  --old-trapdoor 17 \
  --new-trapdoor 27 \
  --new-nullifier 87 \
  --merkle-path "15554272943220125889,94010064041599568,7999912732987454829,2345852616018906843:1256404252992470571,7300445624000758769,8072448918251827482,2716780917944101159" \
  --leaf-index 5 \
  --old-note "1599151707901382900,1044218972583631781,138448951130546224,4882485033043503541" \
  --old-token-amount 7 \
  --new-token-amount 10
${CLIAIN} snark-relation verify \
  --verifying-key-file deposit.groth16.vk.bytes \
  --proof-file deposit.groth16.proof.bytes \
  --public-input-file deposit.groth16.public_input.bytes

echo "Testing 'merge' relation"
${CLIAIN} snark-relation generate-keys merge --max-path-len 4
${CLIAIN} snark-relation generate-proof -p merge.groth16.pk.bytes merge \
  --max-path-len 4 \
  --token-id 1 \
  --first-old-nullifier 19 \
  --second-old-nullifier 29 \
  --new-note "16873374675910556188,7520532372021931293,2252601191743571102,1137408349476470620" \
  --merkle-root "7192045139731542264,1161130572820032929,11248828078021730941,7948961262277222795" \
  --first-old-trapdoor 17 \
  --second-old-trapdoor 23 \
  --new-trapdoor 27 \
  --new-nullifier 87 \
  --first-merkle-path "15554272943220125889,94010064041599568,7999912732987454829,2345852616018906843:12424795876863646335,17547963776984128507,5386482043686570840,8102841056232622473" \
  --second-merkle-path "16612482296235189599,15296027520069836296,4688001093087614490,6035486416775057912:7648513976096501199,6194936452710765692,2531672351598095972,5307215101199693779" \
  --first-leaf-index 5 \
  --second-leaf-index 6 \
  --first-old-note "11425106013701053451,11840150414921744375,7525412728913434437,7906055471841466594" \
  --second-old-note "17914626646753717819,12876986036806086814,4826170774187111240,739180176178530409" \
  --first-old-token-amount 3 \
  --second-old-token-amount 7 \
  --new-token-amount 10
${CLIAIN} snark-relation verify \
  --verifying-key-file merge.groth16.vk.bytes \
  --proof-file merge.groth16.proof.bytes \
  --public-input-file merge.groth16.public_input.bytes

echo "Testing 'withdraw' relation"
${CLIAIN} snark-relation generate-keys withdraw --max-path-len 4
${CLIAIN} snark-relation generate-proof -p withdraw.groth16.pk.bytes withdraw \
  --max-path-len 4 \
  --old-nullifier 19 \
  --merkle-root "5411471078697670106,10647428736348491478,1017459162637849567,3428828460234970929" \
  --new-note "15103081147574579484,3442523252788861192,7376749974720940667,1643089669551520218" \
  --token-id 1 \
  --token-amount-out 7 \
  --fee 1 \
  --recipient "212,53,147,199,21,253,211,28,97,20,26,189,4,169,159,214,130,44,133,88,133,76,205,227,154,86,132,231,165,109,162,125" \
  --old-trapdoor 17 \
  --new-trapdoor 27 \
  --new-nullifier 87 \
  --merkle-path "15554272943220125889,94010064041599568,7999912732987454829,2345852616018906843:1256404252992470571,7300445624000758769,8072448918251827482,2716780917944101159" \
  --leaf-index 5 \
  --old-note "2257517311045912551,9329547706917600007,17678219388335595033,2758194574870438734" \
  --whole-token-amount 10 \
  --new-token-amount 3

${CLIAIN} snark-relation verify \
  --verifying-key-file withdraw.groth16.vk.bytes \
  --proof-file withdraw.groth16.proof.bytes \
  --public-input-file withdraw.groth16.public_input.bytes
