const { WebSocket } = require('ws');
const msgpack = require('@msgpack/msgpack');

const BOTS = 100;
const SERVER_URL = 'ws://127.0.0.1:9001/ws'; // Correct endpoint

let active = 0;
for (let i = 0; i < BOTS; i++) {
    const ws = new WebSocket(SERVER_URL);
    ws.binaryType = "arraybuffer";

    ws.on('open', () => {
        // Send join
        active++;
        const joinMsg = { Join: { token: `bot_${i}` } };
        ws.send(msgpack.encode(joinMsg));

        let seq = 1;
        setInterval(() => {
            if (ws.readyState === WebSocket.OPEN) {
                const moveMsg = { Move: { direction: { x: Math.random() * 2 - 1, y: Math.random() * 2 - 1 }, seq: seq++ } };
                ws.send(msgpack.encode(moveMsg));
            }
        }, 50); // 20 times per sec
    });

    ws.on('error', (err) => {
        console.error(`Bot ${i} error:`, err.message);
    });
}
console.log(`Starting ${BOTS} bots`);
