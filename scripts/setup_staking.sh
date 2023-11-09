#!/bin/bash

function usage(){
  cat << EOF
Usage:
  $0
     --cliain-path path
        path to cliain's binary
    --validator-pod-name pod-name
        validator's pod name, e.g. aleph-node-validator-0
    --namespace n
       namespace to use, e.g. devnet
    [--cliain-no-copy]
       by default script copies cliain binary to --validator-pod-name:/tmp/cliain; this flag prevents it
    [--help]
       displays this info
    [--interactive]
        interactive mode, by default disabled. It makes user to press any key in significant phases to confirm state.
    [--staking-config-file file]
        Optional param.
        path to validator's staking config file, which have following format:
           validator's seed
           stash_account_seed
           controller_account_seed
           minimal_validator_bond
           validator's commission
        Example:
           //0
           //Stash0
           //Controller0
           25000
           10
    [--key-rotation file]
       Optional param.
       Runs rotateKeys() and setKeys() for a given validator. file is a config like in --staking-config-file argument.
    [--set-staking-limits "minimal_nominator_bond,minimal_validator_bond,max_nominators_count,max_validators_count"]
       Optional param.
       Command parameter is comma-separated tuple which is an argument to SetStakingLimits call (requires sudo account).
    [--change-validators account_ids_comma_separated]
        Optional param.
        Calls ChangeMembers under the hood with given list of validators (requires sudo account).
    [--force-new-era]
       Optional param.
       Calls ForceNewEra call under the hood (requires sudo account).
    [--bond-nominate-file file]
       Optional param.
       path to nominate-bond config file, which have following format:
          rich's account seed
          nominator's seed
          nominate stake in tokens
          nominee's account
       The script does following things:
        1. It transfers given stake in tokens from rich's account to nominator's account
        2. It bonds nominator's account to the same account as controller
        3. It calls nominate on nominator's controller for nominee
      Example file:
        //Rich-account
        //20
        100
        5D34dL5prEUaGNQtPPZ3yN5Y6BnkfXunKXXz6fo7ZJbLwRRH
EOF
  exit 0
}

source ./scripts/common.sh

trap sigint_trap SIGINT

while [[ $# -gt 0 ]]; do
  case $1 in
    --cliain-path)
      CLIAIN_PATH="$2"
      shift;shift
      ;;
    --staking-config-file)
      STAKING_CONFIG_FILE="$2"
      shift;shift
      ;;
   --key-rotation)
      KEY_ROTATION="$2"
      shift;shift
      ;;
    --set-staking-limits)
      SET_STAKING_LIMITS="$2"
      shift;shift
      ;;
    --change-validators)
      CHANGE_VALIDATORS="$2"
      shift;shift
      ;;
    --validator-pod-name)
      VALIDATOR_POD_NAME="$2"
      shift;shift
      ;;
    --cliain-no-copy)
      CLIAIN_NO_COPY="YES"
      shift
      ;;
     --force-new-era)
      FORCE_NEW_ERA="YES"
      shift
      ;;
     --interactive)
      INTERACTIVE="YES"
      shift
      ;;
    --help)
      usage
      shift
      ;;
    --namespace)
      NAMESPACE="$2"
      shift;shift
      ;;
    --bond-nominate-file)
      BOND_NOMINATE="$2"
      shift;shift
      ;;
    *)
      error "Unrecognized argument $1!"
      ;;
  esac
done

function get_ss58_address_from_seed() {
  seed="$1"
  cliain_path="$2"

  echo "$("${cliain_path}" --seed "${seed}" seed-to-ss58  2>&1 | grep "SS58 Address:" | awk '{print $6;}')"
}

function prompt_if_interactive_mode() {
  msg="$1"
  if [ -n "${INTERACTIVE}" ]; then
    read -p "$msg"
  fi
}

function transfer_tokens() {
  signer_account_seed="$1"
  to_account="$2"
  tokens="$3"
  namespace="$4"
  validator_pod_name="$5"

  cmd_on_pod=(
    "${CLIAIN_PATH_ON_POD}"
     --node 127.0.0.1:9944
     --seed "${signer_account_seed}"
      transfer
        --amount-in-tokens "${tokens}"
        --to-account "${to_account}"
  )
  kubectl exec --stdin --tty -n "${namespace}" "${validator_pod_name}" -- "${cmd_on_pod[@]}" || \
    error "Failed to run command on pod ${cmd_on_pod[@]}"
  prompt_if_interactive_mode "Press enter to continue"
}

function bond() {
  signer_account_seed="$1"
  controller_account="$2"
  stake_tokens="$3"
  namespace="$4"
  validator_pod_name="$5"

  cmd_on_pod=(
    "${CLIAIN_PATH_ON_POD}"
      --node 127.0.0.1:9944
      --seed "${signer_account_seed}"
        bond
          --controller-account "${controller_account}"
          --initial-stake-tokens "${stake_tokens}"
  )
  kubectl exec --stdin --tty -n "${namespace}" "${validator_pod_name}" -- "${cmd_on_pod[@]}" || \
    error "Failed to run command on pod ${cmd_on_pod[@]}"
  prompt_if_interactive_mode "Press enter to continue"
}

function rotate_keys() {
  validator_seed="$1"
  namespace="$2"
  validator_pod_name="$3"

   cmd_on_pod=(
      "${CLIAIN_PATH_ON_POD}"
       --node 127.0.0.1:9944
       --seed "${validator_seed}"
        rotate-keys
   )

	rotate_keys_output=$(kubectl exec --stdin --tty -n "${namespace}" "${validator_pod_name}" -- "${cmd_on_pod[@]}" 2>&1)
	new_keys=$(echo "${rotate_keys_output}" | grep "Rotated keys:" | awk '{print $7;}' | tr -d "\"" | tr -d "\\r")
	info "new keys are ${new_keys}"
	prompt_if_interactive_mode "Press enter to continue"
}

function set_keys() {
  validator_seed="$1"
  new_keys="$2"
  namespace="$3"
  validator_pod_name="$4"

  cmd_on_pod=(
    "${CLIAIN_PATH_ON_POD}"
      --node 127.0.0.1:9944
      --seed "${validator_seed}"
        set-keys
          --new-keys "${new_keys}"
  )
  kubectl exec --stdin --tty -n "${namespace}" "${validator_pod_name}" -- "${cmd_on_pod[@]}" || \
    error "Failed to run command on pod ${cmd_on_pod[@]}"
	prompt_if_interactive_mode "Press enter to continue"
}

function validate() {
  validator_controller_seed="$1"
  commission="$2"
  namespace="$3"
  validator_pod_name="$4"

  cmd_on_pod=(
    "${CLIAIN_PATH_ON_POD}"
      --node 127.0.0.1:9944
       --seed "${validator_controller_seed}"
        validate
           --commission-percentage "${commission}"
  )
  kubectl exec --stdin --tty -n "${namespace}" "${validator_pod_name}" -- "${cmd_on_pod[@]}" || \
    error "Failed to run command on pod ${cmd_on_pod[@]}"
  prompt_if_interactive_mode "Press enter to continue"
}

function nominate() {
  nominator_seed="$1"
  nominee="$2"
  namespace="$3"
  validator_pod_name="$4"

  cmd_on_pod=(
    "${CLIAIN_PATH_ON_POD}"
      --node 127.0.0.1:9944
       --seed "${nominator_seed}"
        nominate
           --nominee "${nominee}"
  )
  kubectl exec --stdin --tty -n "${namespace}" "${validator_pod_name}" -- "${cmd_on_pod[@]}" || \
    error "Failed to run command on pod ${cmd_on_pod[@]}"
  prompt_if_interactive_mode "Press enter to continue"
}

function set_staking_limits() {
  minimal_nominator_bond="$1"
  minimal_validator_bond="$2"
  max_nominators_count="$3"
  max_validators_count="$4"
  validator_pod_name="$5"
  namespace="$6"

  info "Calling setStakingLimits() with minimal nominator bond ${minimal_nominator_bond}, \
minimal validator bond ${minimal_validator_bond} \
max nominators count ${max_nominators_count} \
max validators count ${max_validators_count}"
  prompt_if_interactive_mode "Press enter to continue"

  cmd_on_pod=(
    "${CLIAIN_PATH_ON_POD}"
      --node 127.0.0.1:9944
        set-staking-limits
         --minimal-nominator-stake "${minimal_nominator_bond}"
         --minimal-validator-stake "${minimal_validator_bond}"
         --max-nominators-count "${max_nominators_count}"
         --max-validators-count "${max_validators_count}"
  )
  # workaround for cliain expecting root account seed from stdin
  info "Provide seed for root account:"
  kubectl exec --stdin --tty -n "${namespace}" "${validator_pod_name}" -- "${cmd_on_pod[@]}" || \
      error "Failed to run command on pod ${cmd_on_pod[@]}"
  prompt_if_interactive_mode "Press enter to continue"
}

function change_validators() {
  new_members="$1"
  validator_pod_name="$2"
  namespace="$3"

  new_members_split=$(echo "${new_members}" | tr ',' '\n')
  info "Calling changeMembers() with new members"
  echo "${new_members_split}"
  prompt_if_interactive_mode "Press enter to continue"

  cmd_on_pod=(
    "${CLIAIN_PATH_ON_POD}"
      --node 127.0.0.1:9944
        change-validators
          --validators "${new_members}"
    )
  # workaround for cliain expecting root account seed from stdin
  info "Provide seed for root account:"
  kubectl exec --stdin --tty -n "${namespace}" "${validator_pod_name}" -- "${cmd_on_pod[@]}" || \
      error "Failed to run command on pod ${cmd_on_pod[@]}"
  prompt_if_interactive_mode "Press enter to continue"
}

function force_new_era() {
  validator_pod_name="$1"
  namespace="$2"

  info "Calling forceNewEra()"
  prompt_if_interactive_mode "Press enter to continue"

  cmd_on_pod=(
    "${CLIAIN_PATH_ON_POD}"
      --node 127.0.0.1:9944
        force-new-era
    )
  # workaround for cliain expecting root account seed from stdin
  info "Provide seed for root account:"
  kubectl exec --stdin --tty -n "${namespace}" "${validator_pod_name}" -- "${cmd_on_pod[@]}" || \
      error "Failed to run command on pod ${cmd_on_pod[@]}"
  prompt_if_interactive_mode "Press enter to continue"
}

function copy_cliain_to_pod() {
  cliain_path="$1"
  validator_pod_name="$2"
  namespace="$3"

  info "Copying binary to validator's pod ${validator_pod_name}:${CLIAIN_PATH_ON_POD}"
  kubectl cp -n "${namespace}" "${cliain_path}" "${validator_pod_name}":"${CLIAIN_PATH_ON_POD}" || \
    error "Failed to copy cliain binary to ${validator_pod_name}:${CLIAIN_PATH_ON_POD}"
  prompt_if_interactive_mode "Press enter to continue"
}

function run_key_rotation() {
  staking_config_file="$1"
  cliain_path="$2"
  namespace="$3"
  validator_pod_name="$4"

  validator_seed=$(sed '1q;d' "${staking_config_file}")
  validator_controller_seed=$(sed '3q;d' "${staking_config_file}")

  controller_public_key=$(get_ss58_address_from_seed "${validator_controller_seed}" "${cliain_path}")

  prompt_if_interactive_mode "Press enter to continue"
  rotate_keys "${validator_seed}" "${namespace}" "${validator_pod_name}"

  info "Setting keys on controller account ${controller_public_key}"
  prompt_if_interactive_mode "Press enter to continue"
  set_keys "${validator_controller_seed}" "${new_keys}" "${namespace}" "${validator_pod_name}"
}

function run_validator_setup() {
  staking_config_file="$1"
  cliain_path="$2"
  validator_pod_name="$3"
  namespace="$4"

  validator_seed=$(sed '1q;d' "${staking_config_file}")
  validator_stash_seed=$(sed '2q;d' "${staking_config_file}")
  validator_controller_seed=$(sed '3q;d' "${staking_config_file}")
  minimal_validator_bond=$(sed '4q;d' "${staking_config_file}")
  validator_commission=$(sed '5q;d' "${staking_config_file}")

  validator_public_key=$(get_ss58_address_from_seed "${validator_seed}" "${cliain_path}")
  stash_public_key=$(get_ss58_address_from_seed "${validator_stash_seed}" "${cliain_path}")
  controller_public_key=$(get_ss58_address_from_seed "${validator_controller_seed}" "${cliain_path}")

  info "Setting up validator config on pod ${validator_pod_name} with following settings:"
  info "Validator's account id is ${validator_public_key}"
  info "Validator's stash account id is ${stash_public_key}"
  info "Validator's controller key is ${controller_public_key}"
  info "Validator's minimal stake: ${minimal_validator_bond}"
  info "Validator's commission: ${validator_commission}"
  prompt_if_interactive_mode "Press enter to continue"

  # one token more to cover tx fees
  stash_tokens=$((minimal_validator_bond + 1))
  info "Transferring ${stash_tokens} tokens from validator's account ${validator_public_key} to ${stash_public_key}"
  prompt_if_interactive_mode "Press enter to continue"
  transfer_tokens "${validator_seed}" "${stash_public_key}" "${stash_tokens}" "${namespace}" "${validator_pod_name}"

  controller_tokens="1"
	info "Transferring ${controller_tokens} tokens from validator's account ${validator_public_key} to ${controller_public_key}"
	prompt_if_interactive_mode "Press enter to continue"
  transfer_tokens "${validator_seed}" "${controller_public_key}" "${controller_tokens}" "${namespace}" "${validator_pod_name}"

  info "Bonding stash account ${stash_public_key} with controller account ${controller_public_key}"
  prompt_if_interactive_mode "Press enter to continue"
  bond "${validator_stash_seed}" "${controller_public_key}" "${minimal_validator_bond}" "${namespace}" "${validator_pod_name}"

  info "Rotating keys for validator ${validator_public_key}"
  run_key_rotation "${staking_config_file}" "${cliain_path}" "${namespace}" "${validator_pod_name}"

	info "Calling validate on controller account ${controller_public_key}"
	prompt_if_interactive_mode "Press enter to continue"
	validate "${validator_controller_seed}" "${validator_commission}" "${namespace}" "${validator_pod_name}"
}

function bond_nominate {
  bond_nominate_file="$1"
  cliain_path="$2"
  validator_pod_name="$3"
  namespace="$4"

  rich_account_seed=$(sed '1q;d' "${bond_nominate_file}")
  nominator_seed=$(sed '2q;d' "${bond_nominate_file}")
  nominator_stake_tokens=$(sed '3q;d' "${bond_nominate_file}")
  nominee_account=$(sed '4q;d' "${bond_nominate_file}")

  rich_account_id=$(get_ss58_address_from_seed "${rich_account_seed}" "${cliain_path}")
  nominator_account_id=$(get_ss58_address_from_seed "${nominator_seed}" "${cliain_path}")

  info "Setting up nominator on pod ${validator_pod_name} with following settings:"
  info "Nominator's account id is ${nominator_account_id}"
  info "Nominator's stake is ${nominator_stake_tokens}"
  info "Rich's account id is ${rich_account_id}"
  prompt_if_interactive_mode "Press enter to continue"

  # one token more to cover tx fees for bond and nominate
  tokens_to_transfer=$((nominator_stake_tokens + 1))
  info "Transferring ${tokens_to_transfer} tokens from rich's account ${rich_account_id} to nominator ${nominator_account_id}"
  prompt_if_interactive_mode "Press enter to continue"
  transfer_tokens "${rich_account_seed}" "${nominator_account_id}" "${tokens_to_transfer}" "${namespace}" "${validator_pod_name}"

  info "Bonding nominator account ${nominator_account_id} with the same account as controller"
  prompt_if_interactive_mode "Press enter to continue"
  bond "${nominator_seed}" "${nominator_account_id}" "${nominator_stake_tokens}" "${namespace}" "${validator_pod_name}"

  info "Calling nominate on nominator controller account ${nominator_account_id} for nominee ${nominee_account}"
  prompt_if_interactive_mode "Press enter to continue"
  nominate "${nominator_seed}" "${nominee_account}" "${namespace}" "${validator_pod_name}"
}

if [ -z "${CLIAIN_PATH}" ]; then
  error "--cliain-path not specified!"
fi
if [ -z "${NAMESPACE}" ]; then
  error "--namespace not specified!"
fi
if [ ! -x "${CLIAIN_PATH}" ]; then
  error "cliain binary not executable!"
fi
if [ -z "${VALIDATOR_POD_NAME}" ]; then
  error "--validator-pod-name not specified!"
fi

# consts globals which are consistent during whole script run
# path on which binary will be copied to on validator's pod
CLIAIN_PATH_ON_POD="/tmp/cliain"

if  [ -z "${CLIAIN_NO_COPY}" ]; then
  did_something="true"
  copy_cliain_to_pod "${CLIAIN_PATH}" "${VALIDATOR_POD_NAME}" "${NAMESPACE}"
fi
if  [ -n "${STAKING_CONFIG_FILE}" ]; then
  did_something="true"
  run_validator_setup "${STAKING_CONFIG_FILE}" "${CLIAIN_PATH}" "${VALIDATOR_POD_NAME}" "${NAMESPACE}"
fi
if  [ -n "${KEY_ROTATION}" ]; then
  did_something="true"
  run_key_rotation "${KEY_ROTATION}" "${CLIAIN_PATH}" "${NAMESPACE}" "${VALIDATOR_POD_NAME}"
fi
if  [ -n "${SET_STAKING_LIMITS}" ]; then
  did_something="true"
  minimal_nominator_bond=$(echo "${SET_STAKING_LIMITS}" | cut -f 1 -d ',')
  minimal_validator_bond=$(echo "${SET_STAKING_LIMITS}" | cut -f 2 -d ',')
  max_nominators_count=$(echo "${SET_STAKING_LIMITS}" | cut -f 3 -d ',')
  max_validators_count=$(echo "${SET_STAKING_LIMITS}" | cut -f 4 -d ',')
  set_staking_limits "${minimal_nominator_bond}" "${minimal_validator_bond}" "${max_nominators_count}" "${max_validators_count}" "${VALIDATOR_POD_NAME}" "${NAMESPACE}"
fi
if  [ -n "${BOND_NOMINATE}" ]; then
  did_something="true"
  bond_nominate "${BOND_NOMINATE}" "${CLIAIN_PATH}" "${VALIDATOR_POD_NAME}" "${NAMESPACE}"
fi
if  [ -n "${CHANGE_VALIDATORS}" ]; then
  did_something="true"
  change_validators "${CHANGE_VALIDATORS}" "${VALIDATOR_POD_NAME}" "${NAMESPACE}"
fi
if  [ -n "${FORCE_NEW_ERA}" ]; then
  did_something="true"
  force_new_era "${VALIDATOR_POD_NAME}" "${NAMESPACE}"
fi

if [ -z "${did_something}" ]; then
  warning "Did nothing, did you forget to pass some flags?"
fi
