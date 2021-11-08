# Monitoring running nodes

## Installation and usage

_Remark: The instructions are valid when working in the `benchmark` directory._

1. You will need `docker` (version `20.10+`) and `docker-compose` installed.
2. Install `fabric` and `parallel`.
3. Specify where Prometheus should be fetching metrics from, i.e. add IPs of the machines running the protocol 
(together with the port, usually `9615`) to the `targets` entry in `prometheus.yml`, e.g.:
```yml
...
  - targets: [
      "host.docker.internal:9615",
      "01.234.56.789:9615",
      "12.345.67.890:9615"
    ]
```
_Remark: Use `host.docker.internal` instead of `localhost`_.
3. Run `docker-compose up` (you can add the `-d` flag for the detached mode).
4. View the dashboard at `localhost:3000` in your browser.
5. When the monitoring is in detached mode you can stop it by running `docker-compose down`.

**Important: Run `aleph-node` with `--prometheus-external` flag.**

## Troubleshooting

In case there is no data displayed in Grafana, check the connection between Prometheus server and its targets at 
`localhost:9090/targets`.

If updating `docker` is not an option, replace `extra_hosts` entry with `network_mode: "host"` in `docker-compose.yml`. 
Then use standard `localhost` target instead of `host.docker.internal`.

# Benchmarking aleph-node

## Installation and usage

_Remark: The instructions are valid when working in the `benchmark` directory._

Apart from the prerequisites needed for monitoring, you need also Python 3 interpreter with package installer (`pip`). Then:

1. Run `pip install -r requirements.txt`
2. Create a directory `key_pairs` and put there your ssh keys: both the private key (e.g. `aleph.pem`) and its fingerprint (`aleph.fingerprint`)
3. Setup your AWS access (credentials and a default region)

_Remark: As for now, everything is happening within the default region._

### Running an experiment

```
usage: benchmark.py run [-h] [--tag TAG] nparties aleph_node_binary

positional arguments:
  nparties           number of nodes in the experiment
  aleph_node_binary  aleph-node executable

optional arguments:
  -h, --help            show this help message and exit
  --tag TAG             tag for the machines
  --unit-creation-delay UNIT_CREATION_DELAY
                        corresponding flag to the one for chainspec
```

For example the command below:
```shell
# probably you will have to run `chmod +x benchmark.py` before;
# if either this is not an option or you only have python linked in another way
# (like `python3`), just run it like a standard script, e.g. `python3 benchmark.py ...`

$ ./benchmark.py run 8 ../target/release/aleph-node --tag bench
```

will run the protocol on 8 instances with the binary located in `../target/release/` and their corresponding tag 
(for e.g. security group) will be `bench`. Also, two Docker containers will be run (in the detached mode):
one with Prometheus and the second one with Grafana servers.
They are available at `localhost:9090` and `localhost:3000` respectively.
At the end, the dashboard will be automatically displayed in the default browser.

### Stopping the experiment

```
usage: benchmark.py clean [-h] [--kill-monitoring] tag

positional arguments:
  tag                   tag for the machines

optional arguments:
  -h, --help            show this help message and exit
  --kill-monitoring, -k
                        whether to stop docker with Prometheus and Grafana
```

For example the command below:
```shell
$ ./benchmark.py clean bench
```

will terminate all instances with a tag `bench` (possibly some not related to the experiment). 
It will also remove all the auxiliary files and directories created while preparing the experiment.

To additionally stop the docker containers with Prometheus and Grafana add the corresponding flag:
```shell
$ ./benchmark.py clean bench --kill-monitoring
# or briefly
$ ./benchmark.py clean bench -k
```
