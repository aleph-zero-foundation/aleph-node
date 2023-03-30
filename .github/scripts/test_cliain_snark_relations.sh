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
  --note "5021452170217037840,5937007201264158367,3788707655610774637,3098344940909620309" \
  --token-id 1 \
  --token-amount 100000000000000000000 \
  --trapdoor "17,17,17,17" \
  --nullifier "19,19,19,19"
${CLIAIN} snark-relation verify \
  --verifying-key-file deposit.groth16.vk.bytes \
  --proof-file deposit.groth16.proof.bytes \
  --public-input-file deposit.groth16.public_input.bytes

echo "Testing 'deposit-and-merge' relation"
${CLIAIN} snark-relation generate-keys deposit-and-merge
${CLIAIN} snark-relation generate-proof -p deposit_and_merge.groth16.pk.bytes deposit-and-merge \
  --max-path-len 4 \
  --token-id 1 \
  --old-nullifier "19,19,19,19" \
  --new-note "854657170271966638,6996340921012829024,12720591695142793909,923004647278935725" \
  --token-amount 3 \
  --merkle-root "4970245944095592870,5762575095171205456,5015633725558548065,8090765869186154678" \
  --old-trapdoor "17,17,17,17" \
  --new-trapdoor "27,27,27,27" \
  --new-nullifier "87,87,87,87" \
  --merkle-path "12698325876282272042,17403469436129398226,13982452835839905675,343276455934401664:9789748465458848555,1564882621652095182,8694335469416387030,3080950211769826847" \
  --leaf-index 5 \
  --old-note "2303246616037476515,12412331041488056859,17414466554834162321,6983671662120997071" \
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
  --first-old-nullifier "19,19,19,19" \
  --second-old-nullifier "29,29,29,29" \
  --new-note "854657170271966638,6996340921012829024,12720591695142793909,923004647278935725" \
  --merkle-root "11926928053736259884,15199127768559165609,3123774460421121401,5412547381715700899" \
  --first-old-trapdoor "17,17,17,17" \
  --second-old-trapdoor "23,23,23,23" \
  --new-trapdoor "27,27,27,27" \
  --new-nullifier "87,87,87,87" \
  --first-merkle-path "12698325876282272042,17403469436129398226,13982452835839905675,343276455934401664:15824961935568746152,12099218534744112576,8053798444391922082,375129945584097643" \
  --second-merkle-path "16498175756097396756,7820699626345718754,17740990136768214997,419939589553619136:3213195178338067105,18216711066863347898,9487198089552097856,2592260609346765023" \
  --first-leaf-index 5 \
  --second-leaf-index 6 \
  --first-old-note "17971796357364363031,3992999242947223682,10174163623557873951,5768923177170090214" \
  --second-old-note "6596962889836384374,3067556059759559753,13570499559684082369,2662557797379505726" \
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
  --old-nullifier "19,19,19,19" \
  --merkle-root "13496827316307102639,8879577521823158695,8307123790263675955,4626922397137096381" \
  --new-note "12656086890811094487,15071272367784891170,4936917677635145998,5655781692404846644" \
  --token-id 1 \
  --token-amount-out 7 \
  --fee 1 \
  --recipient "212,53,147,199,21,253,211,28,97,20,26,189,4,169,159,214,130,44,133,88,133,76,205,227,154,86,132,231,165,109,162,125" \
  --old-trapdoor "17,17,17,17" \
  --new-trapdoor "27,27,27,27" \
  --new-nullifier "87,87,87,87" \
  --merkle-path "12698325876282272042,17403469436129398226,13982452835839905675,343276455934401664:9789748465458848555,1564882621652095182,8694335469416387030,3080950211769826847" \
  --leaf-index 5 \
  --old-note "14196518331971167933,9351787441197109504,1607381931683279265,5988678530488103713" \
  --whole-token-amount 10 \
  --new-token-amount 3

${CLIAIN} snark-relation verify \
  --verifying-key-file withdraw.groth16.vk.bytes \
  --proof-file withdraw.groth16.proof.bytes \
  --public-input-file withdraw.groth16.public_input.bytes
