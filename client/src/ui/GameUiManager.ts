import type {
	InventorySnapshot,
	ItemDefinition,
	PowerNetworkSnapshot,
	ProgressionSnapshot,
	RecipeDefinition,
	TraderQuote,
} from "../types/protocol";

type JoinHandler = (payload: { username: string; endpoint: string }) => void;

class GameUiManager {
	private root: HTMLDivElement;
	private menu: HTMLDivElement;
	private hud: HTMLDivElement;
	private chatLog: HTMLDivElement;
	private chatInput: HTMLInputElement;
	private inventory: HTMLDivElement;
	private crafting: HTMLDivElement;
	private progression: HTMLDivElement;
	private onboarding: HTMLDivElement;
	private deathOverlay: HTMLDivElement;
	private onJoin: JoinHandler | null = null;

	constructor() {
		this.root = document.createElement("div");
		this.root.id = "lithos-ui-root";
		this.root.innerHTML = this.template();
		document.body.appendChild(this.root);
		this.installStyles();

		this.menu = this.root.querySelector("#ui-menu") as HTMLDivElement;
		this.hud = this.root.querySelector("#ui-hud") as HTMLDivElement;
		this.chatLog = this.root.querySelector("#ui-chat-log") as HTMLDivElement;
		this.chatInput = this.root.querySelector(
			"#ui-chat-input",
		) as HTMLInputElement;
		this.inventory = this.root.querySelector("#ui-inventory") as HTMLDivElement;
		this.crafting = this.root.querySelector("#ui-crafting") as HTMLDivElement;
		this.progression = this.root.querySelector(
			"#ui-progression",
		) as HTMLDivElement;
		this.onboarding = this.root.querySelector(
			"#ui-onboarding",
		) as HTMLDivElement;
		this.deathOverlay = this.root.querySelector("#ui-death") as HTMLDivElement;

		const joinButton = this.root.querySelector(
			"#ui-join-btn",
		) as HTMLButtonElement;
		joinButton.addEventListener("click", () => {
			const username = (
				this.root.querySelector("#ui-username") as HTMLInputElement
			).value.trim();
			const endpoint = (
				this.root.querySelector("#ui-endpoint") as HTMLInputElement
			).value.trim();
			this.onJoin?.({
				username: username.length > 0 ? username : "guest",
				endpoint: endpoint.length > 0 ? endpoint : "ws://localhost:9001",
			});
		});

		this.hideAllGameplay();
	}

	onJoinRequested(handler: JoinHandler): void {
		this.onJoin = handler;
	}

	showMenu(
		servers: Array<{ name: string; endpoint: string; detail: string }>,
	): void {
		this.menu.style.display = "block";
		const list = this.root.querySelector("#ui-server-list") as HTMLDivElement;
		list.innerHTML = "";
		for (const server of servers) {
			const row = document.createElement("button");
			row.className = "ui-server-row";
			row.textContent = `${server.name} ${server.detail}`;
			row.addEventListener("click", () => {
				(this.root.querySelector("#ui-endpoint") as HTMLInputElement).value =
					server.endpoint;
			});
			list.appendChild(row);
		}
	}

	hideMenu(): void {
		this.menu.style.display = "none";
	}

	showGameplay(): void {
		this.hud.style.display = "block";
		this.inventory.style.display = "block";
		this.crafting.style.display = "block";
		this.progression.style.display = "block";
	}

	hideAllGameplay(): void {
		this.hud.style.display = "none";
		this.inventory.style.display = "none";
		this.crafting.style.display = "none";
		this.progression.style.display = "none";
		this.onboarding.style.display = "none";
		this.deathOverlay.style.display = "none";
	}

	showOnboarding(show: boolean): void {
		this.onboarding.style.display = show ? "block" : "none";
	}

	showDeathOverlay(show: boolean): void {
		this.deathOverlay.style.display = show ? "flex" : "none";
	}

	updateVitals(payload: {
		health: string;
		oxygen: string;
		ammo: string;
		credits: string;
		fps: string;
		tick: string;
	}): void {
		(this.root.querySelector("#ui-health") as HTMLSpanElement).textContent =
			payload.health;
		(this.root.querySelector("#ui-oxygen") as HTMLSpanElement).textContent =
			payload.oxygen;
		(this.root.querySelector("#ui-ammo") as HTMLSpanElement).textContent =
			payload.ammo;
		(this.root.querySelector("#ui-credits") as HTMLSpanElement).textContent =
			payload.credits;
		(this.root.querySelector("#ui-fps") as HTMLSpanElement).textContent =
			payload.fps;
		(this.root.querySelector("#ui-tick") as HTMLSpanElement).textContent =
			payload.tick;
	}

	updateInventory(
		snapshot: InventorySnapshot | null,
		legacyItems: string[] = [],
	): void {
		const target = this.root.querySelector(
			"#ui-inventory-items",
		) as HTMLDivElement;
		const entries = snapshot
			? snapshot.items.map((item) => `${item.item} x${item.quantity}`)
			: legacyItems;
		target.textContent = entries.length > 0 ? entries.join(", ") : "empty";
	}

	updateCraftingCatalog(
		items: ItemDefinition[],
		recipes: RecipeDefinition[],
	): void {
		const itemText = `${items.length} items`;
		const recipeText = `${recipes.length} recipes`;
		(
			this.root.querySelector("#ui-crafting-summary") as HTMLDivElement
		).textContent = `${itemText} | ${recipeText}`;
	}

	updateTraderQuotes(quotes: TraderQuote[]): void {
		const details = quotes
			.slice(0, 3)
			.map(
				(q) =>
					`${q.item} b:${q.buy_price.toFixed(0)} s:${q.sell_price.toFixed(0)} daily:${q.daily_credits_used}/${q.daily_credit_limit}`,
			)
			.join(" | ");
		(
			this.root.querySelector("#ui-trade-summary") as HTMLDivElement
		).textContent = details.length > 0 ? details : "no trader data";
	}

	updateProgression(branches: ProgressionSnapshot[]): void {
		const text = branches
			.map(
				(branch) =>
					`${branch.branch} Lv.${branch.level} ${branch.xp}/${branch.xp_to_next}`,
			)
			.join(" | ");
		(
			this.root.querySelector("#ui-progression-content") as HTMLDivElement
		).textContent = text.length > 0 ? text : "no progression";
	}

	updatePowerState(networks: PowerNetworkSnapshot[]): void {
		(
			this.root.querySelector("#ui-power-summary") as HTMLDivElement
		).textContent =
			networks.length > 0
				? `${networks.length} network(s), load ${networks
						.reduce((sum, net) => sum + net.load_kw, 0)
						.toFixed(0)}kw`
				: "no power network data";
	}

	pushChat(line: string): void {
		const row = document.createElement("div");
		row.textContent = line;
		this.chatLog.appendChild(row);
		this.chatLog.scrollTop = this.chatLog.scrollHeight;
	}

	onChatSubmit(handler: (text: string) => void): void {
		this.chatInput.addEventListener("keydown", (event) => {
			if (event.key !== "Enter") return;
			const text = this.chatInput.value.trim();
			if (!text) return;
			this.chatInput.value = "";
			handler(text);
		});
	}

	private template(): string {
		return `
      <div id="ui-menu">
        <h1>LITHOS</h1>
        <div id="ui-server-list"></div>
        <div class="ui-row"><input id="ui-username" placeholder="callsign"/></div>
        <div class="ui-row"><input id="ui-endpoint" value="ws://localhost:9001"/></div>
        <div class="ui-row"><button id="ui-join-btn">Join</button></div>
      </div>
      <div id="ui-hud">
        <div class="ui-panel">Hull <span id="ui-health">100/100</span></div>
        <div class="ui-panel">O2 <span id="ui-oxygen">100/100</span></div>
        <div class="ui-panel">Ammo <span id="ui-ammo">0/0</span></div>
        <div class="ui-panel">Credits <span id="ui-credits">0</span></div>
        <div class="ui-panel">FPS <span id="ui-fps">0</span> Tick <span id="ui-tick">0</span></div>
        <div class="ui-panel" id="ui-power-summary">no power network data</div>
        <div class="ui-panel" id="ui-trade-summary">no trader data</div>
        <div id="ui-chat">
          <div id="ui-chat-log"></div>
          <input id="ui-chat-input" placeholder="chat..."/>
        </div>
      </div>
      <div id="ui-inventory"><strong>Inventory</strong><div id="ui-inventory-items">empty</div></div>
      <div id="ui-crafting"><strong>Crafting</strong><div id="ui-crafting-summary">0 items | 0 recipes</div></div>
      <div id="ui-progression"><strong>Progression</strong><div id="ui-progression-content">no progression</div></div>
      <div id="ui-onboarding">
        <strong>Onboarding</strong>
        <div>WASD move | Left click fire/mine | Right click interact</div>
        <div>Space warp | C craft | B build | Enter chat</div>
      </div>
      <div id="ui-death">
        <h2>SYSTEM FAILURE</h2>
        <div>Inventory dropped. Press R to respawn.</div>
      </div>
    `;
	}

	private installStyles(): void {
		const style = document.createElement("style");
		style.textContent = `
      #lithos-ui-root { position: fixed; inset: 0; pointer-events: none; color: #e8eaed; font-family: Rajdhani, sans-serif; z-index: 50; }
      #ui-menu { pointer-events: auto; position: absolute; left: 50%; top: 50%; transform: translate(-50%, -50%); width: 420px; background: rgba(13,17,23,0.95); border: 1px solid #2a3a60; padding: 16px; }
      #ui-menu h1 { font-family: Orbitron, monospace; letter-spacing: 8px; color: #58a6ff; margin-bottom: 12px; font-size: 28px; }
      .ui-row { margin-top: 8px; }
      .ui-row input, .ui-row button { width: 100%; background: #0d1117; border: 1px solid #2a3a60; color: #e8eaed; padding: 8px; }
      .ui-server-row { width: 100%; margin-bottom: 4px; text-align: left; background: #101624; border: 1px solid #1e2a45; color: #e8eaed; padding: 6px; }
      #ui-hud { position: absolute; left: 10px; top: 10px; width: 380px; pointer-events: auto; }
      .ui-panel { background: rgba(10,14,26,0.9); border: 1px solid #1e2a45; margin-bottom: 4px; padding: 4px 8px; font-size: 12px; }
      #ui-chat { background: rgba(10,14,26,0.95); border: 1px solid #1e2a45; padding: 6px; }
      #ui-chat-log { height: 130px; overflow-y: auto; font-size: 11px; margin-bottom: 6px; }
      #ui-chat-input { width: 100%; border: 1px solid #2a3a60; background: #0d1117; color: #e8eaed; padding: 4px; pointer-events: auto; }
      #ui-inventory, #ui-crafting, #ui-progression { position: absolute; right: 10px; width: 300px; background: rgba(10,14,26,0.95); border: 1px solid #1e2a45; padding: 8px; pointer-events: auto; font-size: 12px; }
      #ui-inventory { top: 10px; }
      #ui-crafting { top: 120px; }
      #ui-progression { top: 200px; }
      #ui-onboarding { position: absolute; left: 50%; bottom: 16px; transform: translateX(-50%); background: rgba(13,17,23,0.95); border: 1px solid #2a3a60; padding: 8px 12px; font-size: 12px; pointer-events: none; }
      #ui-death { display: none; position: absolute; inset: 0; background: rgba(20, 0, 0, 0.7); color: #ff6666; align-items: center; justify-content: center; flex-direction: column; text-align: center; font-family: Orbitron, monospace; }
      #ui-death h2 { font-size: 38px; margin-bottom: 12px; }
    `;
		document.head.appendChild(style);
	}
}

export const gameUi = new GameUiManager();
