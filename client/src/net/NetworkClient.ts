/**
 * WebSocket network client for communicating with the Lithos game server.
 *
 * Uses MessagePack for binary serialization to match the Rust server codec.
 */

import { decode, encode } from "@msgpack/msgpack";
import type { ClientMessage, ServerMessage } from "../types/protocol";

const DEFAULT_CONNECT_TIMEOUT_MS = 10_000;

export class NetworkClient {
	private ws: WebSocket | null = null;
	private listeners: Array<(msg: ServerMessage) => void> = [];
	private endpoint = "ws://localhost:9001";
	private connectedAtMs = 0;
	private estimatedRttMs = 80;

	setEndpoint(url: string): void {
		this.endpoint = url;
	}

	getEndpoint(): string {
		return this.endpoint;
	}

	isConnected(): boolean {
		return this.ws?.readyState === WebSocket.OPEN;
	}

	getEstimatedRttMs(): number {
		return this.estimatedRttMs;
	}

	/**
	 * Connect to the game server.
	 */
	connect(url = this.endpoint): Promise<void> {
		if (this.isConnected()) {
			return Promise.resolve();
		}

		return new Promise((resolve, reject) => {
			const ws = new WebSocket(url);
			this.ws = ws;
			ws.binaryType = "arraybuffer";

			const timeoutId = window.setTimeout(() => {
				if (ws.readyState !== WebSocket.OPEN) {
					ws.close();
					reject(new Error("WebSocket connection timeout"));
				}
			}, DEFAULT_CONNECT_TIMEOUT_MS);

			ws.onopen = () => {
				window.clearTimeout(timeoutId);
				this.connectedAtMs = Date.now();
				this.endpoint = url;
				console.log("[net] connected to", url);
				resolve();
			};

			ws.onerror = (event) => {
				window.clearTimeout(timeoutId);
				console.error("[net] connection error", event);
				reject(new Error("WebSocket connection failed"));
			};

			ws.onmessage = (event) => {
				const data = new Uint8Array(event.data as ArrayBuffer);
				const msg = decode(data) as ServerMessage;

				if ("Pong" in msg) {
					const now = Date.now();
					const sample = Math.max(1, now - Number(msg.Pong.client_timestamp));
					this.estimatedRttMs = Math.round(
						this.estimatedRttMs * 0.7 + sample * 0.3,
					);
				}

				for (const listener of this.listeners) {
					listener(msg);
				}
			};

			ws.onclose = (event) => {
				window.clearTimeout(timeoutId);
				console.log("[net] disconnected:", event.code, event.reason);
			};
		});
	}

	/**
	 * Send a message to the server.
	 */
	send(msg: ClientMessage): void {
		if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
			console.warn("[net] cannot send - not connected");
			return;
		}
		const bytes = encode(msg);
		this.ws.send(bytes);
	}

	ping(): void {
		this.send({ Ping: { timestamp: Date.now() } });
	}

	/**
	 * Register a listener for incoming server messages.
	 */
	onMessage(callback: (msg: ServerMessage) => void): void {
		this.listeners.push(callback);
	}

	offMessage(callback: (msg: ServerMessage) => void): void {
		this.listeners = this.listeners.filter((listener) => listener !== callback);
	}

	uptimeMs(): number {
		if (this.connectedAtMs === 0) {
			return 0;
		}
		return Date.now() - this.connectedAtMs;
	}

	/**
	 * Disconnect from the server.
	 */
	disconnect(): void {
		this.ws?.close();
		this.ws = null;
		this.connectedAtMs = 0;
	}
}
