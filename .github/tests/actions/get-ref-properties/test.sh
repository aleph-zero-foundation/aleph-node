#!/bin/bash

TMP_PATH=/tmp/gh-action-tests/get-ref-properties
REPO_OWNER=${1:-}
REPO_NAME=${2:-}
DEFAULT_BRANCH=${3:master}
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" &> /dev/null && pwd)


if [[ -z "${REPO_OWNER}" || -z "${REPO_NAME}" ]]; then
  echo "Syntax: test-get-ref-properties.sh <repo_owner> <repo_name>"
  exit 1 
fi

# Ask for continuation
echo "This script will create a directory ${TMP_PATH} and clone ${REPO_OWNER}/${REPO_NAME} repository to it using SSH"
echo "Then it will create few branches and push commit onto them to trigger certain test workflows which should be"
echo "visible in the Actions section of the repository in GitHub UI"

read -p "Press any key to continue... " -n1 -s


rm -rf ${TMP_PATH}
mkdir -p "${TMP_PATH}"
cd ${TMP_PATH}
git clone git@github.com:${REPO_OWNER}/${REPO_NAME}.git .
git checkout ${DEFAULT_BRANCH}

now="$(date +'%Y%m%d%H%m%s')"
branch_prefix="test_ABC-$now"
branch_prefix_flattened="test_ABC-$now"
branch_prefix_argo="test-abc-$now"

for event in commit-push pull-request tag-push; do
  wf="test-get-ref-properties-${event}.yml"
  git checkout ${DEFAULT_BRANCH}
  git checkout -b ${branch_prefix}-${event}

  git rm -rf .github

  mkdir -p .github/actions/get-ref-properties
  mkdir -p .github/workflows
  cp ${SCRIPT_DIR}/../../../actions/get-ref-properties/action.yml .github/actions/get-ref-properties/action.yml
  cp ${SCRIPT_DIR}/${wf} .github/workflows/${wf}

  sed -i '' "s/VALID_OUTPUT_BRANCH/${branch_prefix}-${event}/g" .github/workflows/${wf}
  sed -i '' "s/VALID_OUTPUT_FLATTENED/${branch_prefix_flattened}-${event}/g" .github/workflows/${wf}
  sed -i '' "s/VALID_OUTPUT_ARGO/${branch_prefix_argo}-${event}/g" .github/workflows/${wf}
  sed -i '' "s/VALID_OUTPUT_TAG/tag-${branch_prefix}/g" .github/workflows/${wf}  

  git add .github
  git commit -m "Add workflow for testing ${event} event"
  git push origin ${branch_prefix}-${event}

  if [[ "${event}" == "tag-push" ]]; then
    git tag tag-${branch_prefix}
    git push origin tag-${branch_prefix}
  fi
done

# Ask for action
echo "Create a pull request for branch ${branch_prefix}-pull-request"
echo "Check result of workflows in the ${REPO_OWNER}/${REPO_NAME} repository"

read -p "Press any key once you are finished with checking results to remove the branches" -n1 -s

# Remove temporary branches
git checkout ${DEFAULT_BRANCH}
for event in commit-push pull-request tag-push; do
  git push origin :${branch_prefix}-${event}
  if [[ "${event}" == "tag-push" ]]; then
    git push origin :tag-${branch_prefix}
  fi
done

# Remove temporary directory
echo "Please remove temporary directory of ${TMP_PATH}"
echo "For security reason this script will not run 'rm -rf' command"

