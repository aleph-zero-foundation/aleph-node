This directory gathers useful actions for Github pipelines.

---

## `run-e2e-test`
This action runs a single test from the e2e test suite. It requires a test case, which is the name of the test.
It optionally runs the finalization e2e testcase, which is helpful after some e2e tests to double-check nothing is broken.

### Usage
Sample usage:
```yaml
steps:
  - uses: ./.github/actions/run-e2e-test
    with:
      test-case: finalization
```
