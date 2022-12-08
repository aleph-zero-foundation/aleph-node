const argv = require('node:process').argv;
const inLatency = argv.length <= 2 ? 0 : argv.at(2);
const outLatency = argv.length <= 3 ? 0 : argv.at(3);
console.log("setting in-latency to", inLatency);
console.log("setting out-latency to", outLatency);

const SyntheticNetwork = require('../vendor/synthetic-network/frontend');

async function setLatency(host, port, inLatency, outLatency) {
    const synthnet = new SyntheticNetwork({ hostname: host, port: port });
    synthnet.default_link.egress.latency(outLatency);
    synthnet.default_link.ingress.latency(inLatency);
    await synthnet.commit();
}

async function run(inLatency, outLatency) {
    for (let it = 0; it < 5; it++) {
        await setLatency('localhost', 3000 + it, inLatency, outLatency);
    }
}

run(inLatency, outLatency);
