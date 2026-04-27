/**
 * WebSocket network client for communicating with the Lithos game server.
 *
 * Uses MessagePack for binary serialization to match the Rust server's codec.
 */

import { decode, encode } from "@msgpack/msgpack";
import type { ClientMessage, ServerMessage } from "../types/protocol";

export class NetworkClient {
	private ws: WebSocket | null = null;
	private listeners: Array<(msg: ServerMessage) => void> = [];

	/**
	 * Connect to the game server.
	 */
	connect(url: string): Promise<void> {
		return new Promise((resolve, reject) => {
			this.ws = new WebSocket(url);
			this.ws.binaryType = "arraybuffer";

			this.ws.onopen = () => {
				console.log("[net] connected to", url);
				resolve();
			};

			this.ws.onerror = (event) => {
				console.error("[net] connection error", event);
				reject(new Error("WebSocket connection failed"));
			};

			this.ws.onmessage = (event) => {
				const data = new Uint8Array(event.data as ArrayBuffer);
				const msg = decode(data) as ServerMessage;
				for (const listener of this.listeners) {
					listener(msg);
				}
			};

			this.ws.onclose = (event) => {
				console.log("[net] disconnected:", event.code, event.reason);
			};
		});
	}

	/**
	 * Send a message to the server.
	 */
	send(msg: ClientMessage): void {
		if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
			console.warn("[net] cannot send — not connected");
			return;
		}
		const bytes = encode(msg);
		this.ws.send(bytes);
	}

	/**
	 * Register a listener for incoming server messages.
	 */
	onMessage(callback: (msg: ServerMessage) => void): void {
		this.listeners.push(callback);
	}

	/**
	 * Disconnect from the server.
	 */
	disconnect(): void {
		this.ws?.close();
		this.ws = null;
	}
}
