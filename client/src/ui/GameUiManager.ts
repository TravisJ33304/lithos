import type {
	InventoryItemStack,
	InventorySnapshot,
	ItemDefinition,
	PowerNetworkSnapshot,
	ProgressionSnapshot,
	RecipeDefinition,
	TraderQuote,
} from "../types/protocol";

type JoinHandler = (payload: { username: string; endpoint: string }) => void;
type LoginHandler = (payload: { username: string; password: string }) => void;
type CraftHandler = (recipe: string) => void;
type TradeHandler = (item: string, quantity: number) => void;

type FlashKind = "info" | "xp" | "error";

interface HotbarItem {
	label: string;
	quantity?: number;
	title?: string;
}

class GameUiManager {
	private root: HTMLDivElement;
	private menu: HTMLDivElement;
	private login: HTMLDivElement;
	private hud: HTMLDivElement;
	private chatLog: HTMLDivElement;
	private chatInput: HTMLInputElement;
	private inventory: HTMLDivElement;
	private crafting: HTMLDivElement;
	private progression: HTMLDivElement;
	private onboarding: HTMLDivElement;
	private deathOverlay: HTMLDivElement;
	private trader: HTMLDivElement;
	private loading: HTMLDivElement;
	private craftingCatalog: {
		items: ItemDefinition[];
		recipes: RecipeDefinition[];
	} = {
		items: [],
		recipes: [],
	};
	private lastInventoryItems: InventoryItemStack[] = [];
	private craftingOpen = false;
	private traderOpenId: number | null = null;
	private onJoin: JoinHandler | null = null;
	private onLogin: LoginHandler | null = null;
	private onCraft: CraftHandler | null = null;
	private onBuy: TradeHandler | null = null;
	private onSell: TradeHandler | null = null;
	private onChat: ((text: string) => void) | null = null;

	constructor() {
		this.root = document.createElement("div");
		this.root.id = "lithos-ui-root";
		this.root.innerHTML = this.template();
		document.body.appendChild(this.root);
		this.installStyles();

		this.menu = this.root.querySelector("#ui-menu") as HTMLDivElement;
		this.login = this.root.querySelector("#ui-login") as HTMLDivElement;
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
		this.trader = this.root.querySelector("#ui-trader") as HTMLDivElement;
		this.loading = this.root.querySelector("#ui-loading") as HTMLDivElement;

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

		const loginButton = this.root.querySelector(
			"#ui-login-btn",
		) as HTMLButtonElement;
		loginButton.addEventListener("click", () => this.submitLogin());
		(this.root.querySelector("#username") as HTMLInputElement).addEventListener(
			"keydown",
			(event) => {
				if (event.key === "Enter") this.submitLogin();
			},
		);
		(
			this.root.querySelector("#ui-crafting-close") as HTMLButtonElement
		).addEventListener("click", () => this.setCraftingOpen(false));
		(
			this.root.querySelector("#ui-trader-close") as HTMLButtonElement
		).addEventListener("click", () => this.closeTraderPanel());
		this.chatInput.addEventListener("keydown", (event) => {
			if (event.key !== "Enter") return;
			const text = this.chatInput.value.trim();
			if (!text) return;
			this.chatInput.value = "";
			this.onChat?.(text);
		});

		this.hideAllGameplay();
	}

	onJoinRequested(handler: JoinHandler): void {
		this.onJoin = handler;
	}

	onLoginRequested(handler: LoginHandler): void {
		this.onLogin = handler;
	}

	onCraftRequested(handler: CraftHandler): void {
		this.onCraft = handler;
	}

	onTradeRequested(handlers: { buy: TradeHandler; sell: TradeHandler }): void {
		this.onBuy = handlers.buy;
		this.onSell = handlers.sell;
	}

	showMenu(
		servers: Array<{ name: string; endpoint: string; detail: string }>,
	): void {
		this.hideAllGameplay();
		this.login.style.display = "none";
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
				for (const sibling of list.querySelectorAll(".ui-server-row")) {
					sibling.classList.remove("selected");
				}
				row.classList.add("selected");
			});
			list.appendChild(row);
		}
	}

	hideMenu(): void {
		this.menu.style.display = "none";
	}

	showLogin(username: string, endpoint: string): void {
		this.hideAllGameplay();
		this.menu.style.display = "none";
		this.login.style.display = "block";
		(this.root.querySelector("#username") as HTMLInputElement).value = username;
		(
			this.root.querySelector("#ui-login-endpoint") as HTMLSpanElement
		).textContent = endpoint;
		this.setLoginStatus("Ready to launch", "idle");
		window.setTimeout(() => {
			(this.root.querySelector("#username") as HTMLInputElement).focus();
		}, 0);
	}

	hideLogin(): void {
		this.login.style.display = "none";
	}

	setLoginStatus(
		text: string,
		tone: "idle" | "loading" | "error" = "idle",
	): void {
		const status = this.root.querySelector(
			"#ui-login-status",
		) as HTMLDivElement;
		status.textContent = text;
		status.dataset.tone = tone;
		const button = this.root.querySelector(
			"#ui-login-btn",
		) as HTMLButtonElement;
		button.disabled = tone === "loading";
		button.textContent = tone === "loading" ? "CONNECTING" : "LAUNCH";
	}

	showLoading(text: string): void {
		this.loading.style.display = "flex";
		(
			this.root.querySelector("#ui-loading-text") as HTMLDivElement
		).textContent = text;
	}

	hideLoading(): void {
		this.loading.style.display = "none";
	}

	showGameplay(): void {
		this.menu.style.display = "none";
		this.login.style.display = "none";
		this.hud.style.display = "block";
		this.inventory.style.display = "block";
		this.crafting.style.display = "block";
		this.progression.style.display = "block";
		this.root
			.querySelector<HTMLElement>("#ui-crosshair")
			?.removeAttribute("hidden");
	}

	hideAllGameplay(): void {
		this.hud.style.display = "none";
		this.inventory.style.display = "none";
		this.crafting.style.display = "none";
		this.progression.style.display = "none";
		this.trader.style.display = "none";
		this.onboarding.style.display = "none";
		this.deathOverlay.style.display = "none";
		this.root
			.querySelector<HTMLElement>("#ui-crosshair")
			?.setAttribute("hidden", "");
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
		this.updateVital("health", payload.health);
		this.updateVital("oxygen", payload.oxygen);
		this.updateVital("ammo", payload.ammo);
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
		this.lastInventoryItems = snapshot
			? snapshot.items
			: legacyItems.map((item) => ({
					item,
					quantity: 1,
					rarity: "Common",
					category: "Resource",
				}));
		this.renderInventory();
	}

	updateCraftingCatalog(
		items: ItemDefinition[],
		recipes: RecipeDefinition[],
	): void {
		this.craftingCatalog = { items, recipes };
		const itemText = `${items.length} items`;
		const recipeText = `${recipes.length} recipes`;
		(
			this.root.querySelector("#ui-crafting-summary") as HTMLDivElement
		).textContent = `${itemText} | ${recipeText}`;
		this.renderCraftingRecipes();
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
		if (this.traderOpenId !== null) {
			this.renderTraderQuotes(this.traderOpenId, quotes);
		}
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
		const fabrication = branches.find(
			(branch) => branch.branch === "Fabrication",
		);
		const extraction = branches.find(
			(branch) => branch.branch === "Extraction",
		);
		const primary = extraction ?? fabrication ?? branches[0];
		if (primary) {
			(this.root.querySelector("#ui-xp") as HTMLSpanElement).textContent =
				`${primary.branch} Lv.${primary.level}`;
		}
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

	updateSceneContext(title: string, detail: string): void {
		(this.root.querySelector("#ui-scene-title") as HTMLDivElement).textContent =
			title;
		(
			this.root.querySelector("#ui-scene-detail") as HTMLDivElement
		).textContent = detail;
	}

	updateBaseStatus(structures: number): void {
		(
			this.root.querySelector("#ui-base-summary") as HTMLDivElement
		).textContent = `${structures} structure(s) detected`;
	}

	updateHotbar(activeSlot: number, items: HotbarItem[]): void {
		const hotbar = this.root.querySelector("#ui-hotbar") as HTMLDivElement;
		hotbar.innerHTML = "";
		for (let slot = 0; slot <= 9; slot++) {
			const item =
				slot === 0
					? { label: "FIRE", title: "Fire / unarmed" }
					: items[slot - 1];
			const node = document.createElement("div");
			node.className = "ui-hotbar-slot";
			if (slot === activeSlot) node.classList.add("active");
			if (item) node.classList.add("has-item");
			node.title = item?.title ?? "Empty";
			node.innerHTML = `
				<span class="slot-key">${slot}</span>
				<span class="slot-label">${item ? this.abbrev(item.label) : ""}</span>
				<span class="slot-amount">${item?.quantity ?? ""}</span>
			`;
			hotbar.appendChild(node);
		}
	}

	updateMinimap(
		entities: Array<{
			id: number;
			position: { x: number; y: number };
			entity_type: string;
		}>,
		playerId: number,
	): void {
		const svg = this.root.querySelector("#ui-minimap-canvas") as SVGSVGElement;
		const player = entities.find((entity) => entity.id === playerId);
		const px = player?.position.x ?? 0;
		const py = player?.position.y ?? 0;
		const dots = entities.slice(0, 80).map((entity) => {
			const x = Math.max(4, Math.min(146, 75 + (entity.position.x - px) / 32));
			const y = Math.max(4, Math.min(146, 75 + (entity.position.y - py) / 32));
			const color =
				entity.id === playerId
					? "#58a6ff"
					: entity.entity_type === "Hostile"
						? "#ff4422"
						: entity.entity_type === "ResourceNode"
							? "#7a8aa6"
							: entity.entity_type === "Trader"
								? "#2ea043"
								: "#e8eaed";
			return `<circle cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="${entity.id === playerId ? 2.5 : 1.5}" fill="${color}"/>`;
		});
		svg.innerHTML = `<rect width="150" height="150" fill="#0d1117"/>${dots.join("")}`;
	}

	setBuildMode(enabled: boolean): void {
		this.root.classList.toggle("is-build-mode", enabled);
		(this.root.querySelector("#ui-mode") as HTMLSpanElement).textContent =
			enabled ? "BUILD" : "COMBAT";
	}

	setCraftingOpen(open: boolean): void {
		this.craftingOpen = open;
		(
			this.root.querySelector("#ui-crafting-panel") as HTMLDivElement
		).style.display = open ? "block" : "none";
	}

	toggleCraftingPanel(): void {
		this.setCraftingOpen(!this.craftingOpen);
	}

	isCraftingPanelOpen(): boolean {
		return this.craftingOpen;
	}

	pushFlash(message: string, kind: FlashKind = "info"): void {
		const area = this.root.querySelector("#ui-flash-area") as HTMLDivElement;
		const row = document.createElement("div");
		row.className = `ui-flash ${kind}`;
		row.textContent = message;
		area.appendChild(row);
		window.setTimeout(() => row.remove(), 2600);
	}

	pushChat(line: string): void {
		const row = document.createElement("div");
		row.textContent = line;
		row.className = "ui-chat-msg";
		this.chatLog.appendChild(row);
		this.chatLog.scrollTop = this.chatLog.scrollHeight;
	}

	onChatSubmit(handler: (text: string) => void): void {
		this.onChat = handler;
	}

	focusChat(): void {
		this.chatInput.focus();
	}

	openTraderPanel(
		traderId: number,
		quotes: TraderQuote[],
		credits: number,
	): void {
		this.traderOpenId = traderId;
		this.trader.style.display = "block";
		(
			this.root.querySelector("#ui-trader-title") as HTMLDivElement
		).textContent = `TRADER E${traderId}`;
		(
			this.root.querySelector("#ui-trader-credits") as HTMLDivElement
		).textContent = `Faction credits: ${credits.toLocaleString()} CR`;
		this.renderTraderQuotes(traderId, quotes);
	}

	closeTraderPanel(): void {
		this.traderOpenId = null;
		this.trader.style.display = "none";
	}

	private template(): string {
		return `
			<div id="ui-menu">
				<div class="ui-title-block">
					<h1>LITHOS</h1>
					<div>MULTIPLAYER SURVIVAL CRAFTING</div>
				</div>
				<div class="ui-menu-panel">
					<div class="ui-tabs"><button class="active">PLAY</button><button>SETTINGS</button></div>
					<div id="ui-server-list"></div>
					<div class="ui-row"><label>CALLSIGN</label><input id="ui-username" placeholder="callsign#faction"/></div>
					<div class="ui-row"><label>WS</label><input id="ui-endpoint" value="ws://localhost:9001"/></div>
					<div class="ui-row"><button id="ui-join-btn">LAUNCH</button></div>
				</div>
			</div>
			<div id="ui-login">
				<div class="ui-menu-panel">
					<div class="ui-panel-title">LITHOS LAUNCH</div>
					<div class="ui-row"><label>CALLSIGN / EMAIL</label><input id="username" placeholder="callsign or email"/></div>
					<div class="ui-row"><label>PASSWORD</label><input id="ui-password" type="password" placeholder="Supabase password (optional for dev)"/></div>
					<div class="ui-row ui-muted">Endpoint <span id="ui-login-endpoint">ws://localhost:9001</span></div>
					<button id="ui-login-btn">LAUNCH</button>
					<div id="ui-login-status" data-tone="idle">Ready to launch</div>
				</div>
			</div>
			<div id="ui-loading">
				<div class="ui-spinner"></div>
				<div id="ui-loading-text">Loading</div>
			</div>
			<div id="ui-hud">
				<div id="ui-vitals">
					${this.vitalTemplate("health", "HULL", "100/100")}
					${this.vitalTemplate("oxygen", "O2", "100/100")}
					${this.vitalTemplate("ammo", "AMMO", "0/0")}
				</div>
				<div id="ui-status-row">
					<div class="ui-status-chip">XP <span id="ui-xp">--</span></div>
					<div class="ui-status-chip">CR <span id="ui-credits">0</span></div>
					<div class="ui-status-chip">TICK <span id="ui-tick">0</span></div>
					<div class="ui-status-chip">FPS <span id="ui-fps">0</span></div>
					<div class="ui-status-chip">MODE <span id="ui-mode">COMBAT</span></div>
				</div>
				<div id="ui-minimap"><svg id="ui-minimap-canvas" viewBox="0 0 150 150"></svg><span>MINIMAP</span></div>
				<div id="ui-scene-card">
					<div id="ui-scene-title">OVERWORLD</div>
					<div id="ui-scene-detail">WASD move | Left click fire/mine | Right click interact</div>
					<div id="ui-power-summary">no power network data</div>
					<div id="ui-trade-summary">no trader data</div>
					<div id="ui-base-summary">no base structure data</div>
				</div>
				<div id="ui-chat">
					<div id="ui-chat-log"></div>
					<div id="ui-chat-input-line"><span>&gt;</span><input id="ui-chat-input" placeholder="chat..."/></div>
				</div>
				<div id="ui-hotbar"></div>
				<div id="ui-info">[SPACE] Zone Transfer [C] Fabricator [B] Build [0-9] Hotbar [ENTER] Chat</div>
				<div id="ui-flash-area"></div>
			</div>
			<div id="ui-crosshair"><div></div></div>
			<div id="ui-inventory">
				<div class="ui-panel-title">CARGO HOLD <span id="ui-inventory-capacity">0 / 30 SLOTS</span></div>
				<div id="ui-inventory-grid"></div>
			</div>
			<div id="ui-crafting">
				<div class="ui-panel-title">FABRICATOR</div>
				<button id="ui-crafting-toggle" type="button">Press C to open</button>
				<div id="ui-crafting-summary">0 items | 0 recipes</div>
			</div>
			<div id="ui-crafting-panel">
				<div class="ui-modal-header"><span>FABRICATOR</span><button id="ui-crafting-close">[X CLOSE]</button></div>
				<div id="ui-crafting-recipes"></div>
			</div>
			<div id="ui-progression"><div class="ui-panel-title">PROGRESSION</div><div id="ui-progression-content">no progression</div></div>
			<div id="ui-trader">
				<div class="ui-modal-header"><span id="ui-trader-title">TRADER</span><button id="ui-trader-close">[X CLOSE]</button></div>
				<div id="ui-trader-credits">Faction credits: 0 CR</div>
				<div id="ui-trader-quotes"></div>
			</div>
			<div id="ui-onboarding">
				<strong>ONBOARDING</strong>
				<div>WASD move | Left click fire/mine | Right click interact</div>
				<div>Space warp | C fabricator | B build | Enter chat</div>
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
      :root { --bg-deep:#060a14; --bg-panel:#101624; --bg-surface:#151d30; --bg-hover:#1a2640; --border-main:#1e2a45; --border-accent:#2a3a60; --text-primary:#e8eaed; --text-secondary:#7a8aa6; --text-dim:#4a5a70; --highlight:#58a6ff; --danger:#dd3333; --warning:#ffaa33; --safe:#2ea043; --health:#dd3333; --oxygen:#33aaff; --ammo:#f0a000; }
      #lithos-ui-root { position: fixed; inset: 0; pointer-events: none; color: var(--text-primary); font-family: Rajdhani, sans-serif; z-index: 50; user-select: none; }
      #ui-menu, #ui-login { pointer-events: auto; position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 22px; background: radial-gradient(ellipse at 30% 30%, rgba(10,18,40,0.9), rgba(4,8,18,0.94)); }
      .ui-title-block { text-align: center; }
      .ui-title-block h1 { font-family: Orbitron, monospace; letter-spacing: 12px; color: var(--highlight); margin: 0; font-size: 64px; }
      .ui-title-block div { color: var(--text-dim); letter-spacing: 5px; font-size: 13px; }
      .ui-menu-panel { width: 500px; background: rgba(13,17,23,0.95); border: 1px solid var(--border-accent); padding: 16px; position: relative; }
      .ui-menu-panel::before, #ui-inventory::before, #ui-crafting-panel::before, #ui-trader::before { content:""; position:absolute; top:0; left:0; right:0; height:2px; background: linear-gradient(90deg, var(--highlight), transparent 80%); }
      .ui-tabs { display: flex; margin: -16px -16px 12px; border-bottom: 1px solid var(--border-main); }
      .ui-tabs button { flex: 1; background: transparent; border: 0; border-bottom: 2px solid transparent; color: var(--text-dim); padding: 10px; font: 700 10px Orbitron, monospace; letter-spacing: 2px; }
      .ui-tabs button.active { color: var(--highlight); border-bottom-color: var(--highlight); background: rgba(88,166,255,0.06); }
      .ui-row { margin-top: 10px; display: flex; flex-direction: column; gap: 4px; font-size: 10px; color: var(--text-dim); letter-spacing: 1px; }
      .ui-row input, .ui-row button, #ui-login-btn, #ui-crafting-toggle { width: 100%; background: rgba(0,0,0,0.4); border: 1px solid var(--border-main); color: var(--text-primary); padding: 10px; font: 12px "JetBrains Mono", monospace; }
      .ui-row button, #ui-login-btn, #ui-crafting-toggle { background: var(--highlight); border: 0; font-family: Orbitron, monospace; font-weight: 700; letter-spacing: 2px; cursor: pointer; }
      .ui-server-row { width: 100%; margin-bottom: 4px; text-align: left; background: transparent; border: 1px solid var(--border-main); color: var(--text-primary); padding: 8px 10px; cursor: pointer; }
      .ui-server-row:hover, .ui-server-row.selected { background: var(--bg-hover); border-color: var(--highlight); }
      #ui-login-status { margin-top: 10px; font: 10px "JetBrains Mono", monospace; color: var(--text-secondary); }
      #ui-login-status[data-tone="error"] { color: var(--danger); }
      #ui-login-status[data-tone="loading"] { color: var(--warning); }
      #ui-loading { display: none; pointer-events: auto; position: absolute; inset: 0; background: rgba(6,10,20,0.9); align-items: center; justify-content: center; flex-direction: column; gap: 16px; font: 700 11px Orbitron, monospace; letter-spacing: 3px; color: var(--text-dim); }
      .ui-spinner { width: 46px; height: 46px; border: 2px solid var(--border-main); border-top-color: var(--highlight); animation: ui-spin 0.8s linear infinite; }
      @keyframes ui-spin { to { transform: rotate(360deg); } }
      #ui-hud { position: absolute; inset: 0; display: none; }
      #ui-vitals { position: absolute; top: 12px; left: 12px; width: 250px; display: flex; flex-direction: column; gap: 4px; }
      .ui-vital-row { display: flex; align-items: center; gap: 8px; background: rgba(10,14,26,0.85); border: 1px solid var(--border-main); padding: 4px 8px; }
      .ui-vital-label { width: 42px; font: 700 9px Orbitron, monospace; color: var(--text-secondary); letter-spacing: 1px; }
      .ui-vital-track { flex: 1; height: 12px; background: rgba(255,255,255,0.06); border: 1px solid var(--border-main); overflow: hidden; }
      .ui-vital-fill { height: 100%; width: 100%; transition: width 0.2s ease; }
      .ui-vital-fill.health { background: var(--health); }
      .ui-vital-fill.oxygen { background: var(--oxygen); }
      .ui-vital-fill.ammo { background: var(--ammo); }
      .ui-vital-value { width: 55px; text-align: right; font: 700 11px "JetBrains Mono", monospace; }
      #ui-status-row { position: absolute; top: 108px; left: 12px; display: flex; gap: 4px; flex-wrap: wrap; max-width: 620px; }
      .ui-status-chip { background: rgba(10,14,26,0.85); border: 1px solid var(--border-main); padding: 2px 8px; font: 10px "JetBrains Mono", monospace; color: var(--text-secondary); }
      .ui-status-chip span { color: var(--text-primary); font-weight: 700; }
      #ui-minimap { position: absolute; top: 12px; right: 12px; width: 150px; height: 150px; background: rgba(10,14,26,0.9); border: 1px solid var(--border-accent); }
      #ui-minimap svg { width: 100%; height: 100%; }
      #ui-minimap span { position: absolute; bottom: 2px; left: 4px; font: 7px Orbitron, monospace; letter-spacing: 2px; color: var(--text-dim); }
      #ui-scene-card { position: absolute; top: 170px; left: 12px; width: 360px; background: rgba(10,14,26,0.86); border: 1px solid var(--border-main); padding: 8px; font-size: 11px; color: var(--text-secondary); }
      #ui-scene-title, .ui-panel-title { font: 700 11px Orbitron, monospace; letter-spacing: 2px; color: var(--highlight); margin-bottom: 6px; }
      #ui-chat { position: absolute; left: 12px; bottom: 72px; width: 380px; pointer-events: auto; }
      #ui-chat-log { min-height: 58px; max-height: 120px; overflow-y: auto; background: rgba(10,14,26,0.85); border: 1px solid var(--border-main); padding: 6px 8px; font: 10px "JetBrains Mono", monospace; }
      #ui-chat-input-line { display: flex; gap: 5px; align-items: center; margin-top: 2px; background: rgba(10,14,26,0.85); border: 1px solid var(--border-main); padding: 3px 6px; font: 10px "JetBrains Mono", monospace; color: var(--text-dim); }
      #ui-chat-input { flex: 1; background: transparent; border: 0; outline: none; color: var(--text-primary); font: 10px "JetBrains Mono", monospace; }
      #ui-hotbar { position: absolute; bottom: 18px; left: 50%; transform: translateX(-50%); display: flex; gap: 3px; }
      .ui-hotbar-slot { width: 48px; height: 48px; background: rgba(10,14,26,0.88); border: 1px solid var(--border-main); display: flex; align-items: center; justify-content: center; position: relative; font: 700 10px "JetBrains Mono", monospace; color: var(--text-secondary); }
      .ui-hotbar-slot.active { border-color: var(--highlight); background: rgba(88,166,255,0.12); box-shadow: 0 0 8px rgba(88,166,255,0.15); }
      .slot-key { position: absolute; top: 1px; left: 3px; font: 7px Orbitron, monospace; color: var(--text-dim); }
      .slot-amount { position: absolute; right: 3px; bottom: 1px; font-size: 8px; color: var(--text-secondary); }
      #ui-info { position: absolute; bottom: 72px; left: 410px; font: 9px "JetBrains Mono", monospace; color: var(--text-dim); }
      #ui-flash-area { position: absolute; top: 50%; left: 50%; transform: translate(-50%, -80%); display: flex; flex-direction: column; align-items: center; gap: 12px; }
      .ui-flash { font: 700 14px Orbitron, monospace; animation: ui-flash-up 2.5s ease-out forwards; }
      .ui-flash.xp { color: var(--safe); } .ui-flash.error { color: var(--danger); } .ui-flash.info { color: var(--highlight); }
      @keyframes ui-flash-up { 0% { opacity: 1; transform: translateY(0); } 70% { opacity: 1; } 100% { opacity: 0; transform: translateY(-24px); } }
      #ui-crosshair { position: absolute; top: 50%; left: 50%; width: 24px; height: 24px; transform: translate(-50%, -50%); }
      #ui-crosshair[hidden] { display: none; }
      #ui-crosshair::before, #ui-crosshair::after { content:""; position: absolute; background: var(--danger); }
      #ui-crosshair::before { width: 2px; height: 100%; left: 50%; transform: translateX(-50%); }
      #ui-crosshair::after { height: 2px; width: 100%; top: 50%; transform: translateY(-50%); }
      #ui-crosshair div { position: absolute; inset: 3px; border: 2px solid rgba(255,68,34,0.5); border-radius: 50%; }
      .is-build-mode #ui-crosshair::before, .is-build-mode #ui-crosshair::after { background: var(--highlight); }
      .is-build-mode #ui-crosshair div { border-color: rgba(88,166,255,0.65); border-radius: 0; }
      #ui-inventory, #ui-crafting, #ui-progression { position: absolute; right: 10px; width: 340px; background: rgba(16,22,36,0.95); border: 1px solid var(--border-accent); padding: 10px; pointer-events: auto; font-size: 12px; }
      #ui-inventory { top: 10px; } #ui-crafting { top: 306px; } #ui-progression { top: 392px; }
      #ui-inventory-capacity { float: right; color: var(--text-dim); font: 10px "JetBrains Mono", monospace; letter-spacing: 0; }
      #ui-inventory-grid { display: grid; grid-template-columns: repeat(6, 1fr); gap: 4px; }
      .ui-inv-slot { aspect-ratio: 1; background: rgba(0,0,0,0.35); border: 1px solid var(--border-main); display: flex; flex-direction: column; align-items: center; justify-content: center; font: 700 10px "JetBrains Mono", monospace; position: relative; }
      .ui-inv-slot.empty { color: var(--text-dim); opacity: 0.45; }
      .ui-inv-qty { position: absolute; bottom: 2px; right: 3px; font-size: 8px; color: var(--text-secondary); }
      .ui-inv-name { margin-top: 2px; font-size: 6px; color: var(--text-dim); text-transform: uppercase; }
      #ui-crafting-panel, #ui-trader { display: none; position: absolute; left: 50%; top: 50%; transform: translate(-50%, -50%); width: 520px; max-height: 75vh; overflow: auto; background: var(--bg-panel); border: 1px solid var(--border-accent); pointer-events: auto; padding: 14px; }
      .ui-modal-header { display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border-main); padding-bottom: 10px; margin-bottom: 8px; font: 700 13px Orbitron, monospace; letter-spacing: 2px; color: var(--highlight); }
      .ui-modal-header button { background: transparent; border: 1px solid transparent; color: var(--danger); cursor: pointer; font: 11px "JetBrains Mono", monospace; }
      .ui-recipe-card, .ui-trader-row { display: flex; align-items: center; gap: 10px; padding: 7px 10px; border: 1px solid transparent; font: 11px "JetBrains Mono", monospace; }
      .ui-recipe-card:hover, .ui-trader-row:hover { background: var(--bg-hover); border-color: var(--border-main); }
      .ui-recipe-icon { width: 34px; height: 34px; display: flex; align-items: center; justify-content: center; border: 1px solid var(--border-main); color: var(--highlight); }
      .ui-recipe-info, .ui-trader-info { flex: 1; }
      .ui-recipe-inputs, .ui-trader-meta { color: var(--text-secondary); font-size: 10px; margin-top: 2px; }
      .ui-recipe-card button, .ui-trader-row button { border: 1px solid var(--border-main); background: rgba(255,255,255,0.05); color: var(--text-secondary); padding: 4px 8px; cursor: pointer; font: 9px "JetBrains Mono", monospace; }
      .ui-recipe-card button:hover, .ui-trader-row button:hover { border-color: var(--highlight); color: var(--highlight); }
      #ui-onboarding { position: absolute; left: 50%; bottom: 92px; transform: translateX(-50%); background: rgba(13,17,23,0.95); border: 1px solid var(--border-accent); padding: 8px 12px; font-size: 12px; pointer-events: none; }
      #ui-death { display: none; position: absolute; inset: 0; background: rgba(20, 0, 0, 0.7); color: #ff6666; align-items: center; justify-content: center; flex-direction: column; text-align: center; font-family: Orbitron, monospace; pointer-events: auto; }
      #ui-death h2 { font-size: 38px; margin-bottom: 12px; }
    `;
		document.head.appendChild(style);
	}

	private submitLogin(): void {
		const input = this.root.querySelector("#username") as HTMLInputElement;
		const password = (
			this.root.querySelector("#ui-password") as HTMLInputElement
		).value;
		this.onLogin?.({ username: input.value.trim() || "guest", password });
	}

	private vitalTemplate(kind: string, label: string, value: string): string {
		return `<div class="ui-vital-row"><span class="ui-vital-label">${label}</span><div class="ui-vital-track"><div id="ui-${kind}-bar" class="ui-vital-fill ${kind}"></div></div><span id="ui-${kind}" class="ui-vital-value">${value}</span></div>`;
	}

	private updateVital(kind: "health" | "oxygen" | "ammo", value: string): void {
		(this.root.querySelector(`#ui-${kind}`) as HTMLSpanElement).textContent =
			value;
		const [current, max] = value.split("/").map((part) => Number(part));
		const pct =
			Number.isFinite(current) && Number.isFinite(max) && max > 0
				? Math.max(0, Math.min(100, (current / max) * 100))
				: 0;
		(this.root.querySelector(`#ui-${kind}-bar`) as HTMLDivElement).style.width =
			`${pct}%`;
	}

	private renderInventory(): void {
		const grid = this.root.querySelector(
			"#ui-inventory-grid",
		) as HTMLDivElement;
		const capacity = 30;
		grid.innerHTML = "";
		(
			this.root.querySelector("#ui-inventory-capacity") as HTMLSpanElement
		).textContent = `${this.lastInventoryItems.length} / ${capacity} SLOTS`;
		for (let index = 0; index < capacity; index++) {
			const item = this.lastInventoryItems[index];
			const slot = document.createElement("div");
			slot.className = `ui-inv-slot${item ? "" : " empty"}`;
			if (item) {
				slot.title = `${item.item} x${item.quantity}`;
				slot.innerHTML = `<span>${this.abbrev(item.item)}</span><span class="ui-inv-qty">${item.quantity}</span><span class="ui-inv-name">${item.item}</span>`;
			} else {
				slot.textContent = ".";
			}
			grid.appendChild(slot);
		}
	}

	private renderCraftingRecipes(): void {
		const target = this.root.querySelector(
			"#ui-crafting-recipes",
		) as HTMLDivElement;
		target.innerHTML = "";
		for (const recipe of this.craftingCatalog.recipes) {
			const card = document.createElement("div");
			card.className = "ui-recipe-card";
			card.innerHTML = `
				<div class="ui-recipe-icon">${this.abbrev(recipe.output)}</div>
				<div class="ui-recipe-info">
					<div>${recipe.output}</div>
					<div class="ui-recipe-inputs">${recipe.inputs.join(" + ")}</div>
					<div class="ui-recipe-inputs">${recipe.required_branch} Lv.${recipe.required_level}</div>
				</div>
				<button type="button">CRAFT</button>
			`;
			card.querySelector("button")?.addEventListener("click", () => {
				this.onCraft?.(recipe.name);
				this.pushFlash(`${recipe.output} queued`, "info");
			});
			target.appendChild(card);
		}
	}

	private renderTraderQuotes(traderId: number, quotes: TraderQuote[]): void {
		const target = this.root.querySelector(
			"#ui-trader-quotes",
		) as HTMLDivElement;
		const filtered = quotes.filter(
			(quote) => quote.trader_entity_id === traderId,
		);
		target.innerHTML =
			filtered.length === 0
				? `<div class="ui-muted">Waiting for trader quote data...</div>`
				: "";
		for (const quote of filtered) {
			const row = document.createElement("div");
			row.className = "ui-trader-row";
			row.innerHTML = `
				<div class="ui-recipe-icon">${this.abbrev(quote.item)}</div>
				<div class="ui-trader-info">
					<div>${quote.item}</div>
					<div class="ui-trader-meta">daily ${quote.daily_credits_used}/${quote.daily_credit_limit}</div>
				</div>
				<button type="button" data-action="buy">BUY ${Math.floor(quote.sell_price)}</button>
				<button type="button" data-action="sell">SELL ${Math.floor(quote.buy_price)}</button>
			`;
			row
				.querySelector('[data-action="buy"]')
				?.addEventListener("click", () => {
					this.onBuy?.(quote.item, 1);
				});
			row
				.querySelector('[data-action="sell"]')
				?.addEventListener("click", () => {
					this.onSell?.(quote.item, 1);
				});
			target.appendChild(row);
		}
	}

	private abbrev(value: string): string {
		const parts = value.split(/[_\s-]+/).filter(Boolean);
		const letters =
			parts.length > 1
				? parts.map((part) => part[0]).join("")
				: value.replace(/[^a-zA-Z0-9]/g, "").slice(0, 2);
		return (letters || "--").slice(0, 3).toUpperCase();
	}
}

export const gameUi = new GameUiManager();
