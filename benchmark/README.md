# Monitoring nodes

## Installation and usage

1. You will need `docker` (version `20.10+`) and `docker-compose` installed.
2. Specify where Prometheus should be fetching metrics from, i.e. add IPs of the machines running the protocol 
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
