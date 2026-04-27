import type {
	FactionMembership,
	LeaderboardEntry,
	ServerListing,
	SkillBranch,
} from "../types/protocol";

export interface PlayerProfile {
	user_id: string;
	username: string;
	faction: FactionMembership | null;
	progression: Array<{
		branch: SkillBranch;
		level: number;
		xp: number;
		xp_to_next: number;
	}>;
}

export interface FactionSummary {
	faction_id: number;
	name: string;
	wealth: number;
	members: Array<{
		user_id: string;
		username: string;
		role: string;
	}>;
}

export class ApiClient {
	private token: string | null = null;
	private readonly baseUrl: string;

	constructor(baseUrl: string) {
		this.baseUrl = baseUrl;
	}

	setToken(token: string | null): void {
		this.token = token;
	}

	getBaseUrl(): string {
		return this.baseUrl;
	}

	private headers(withJson = false): HeadersInit {
		const headers: Record<string, string> = {};
		if (withJson) {
			headers["Content-Type"] = "application/json";
		}
		if (this.token) {
			headers.Authorization = `Bearer ${this.token}`;
		}
		return headers;
	}

	private async request<T>(path: string, init?: RequestInit): Promise<T> {
		const response = await fetch(`${this.baseUrl}${path}`, {
			...init,
			headers: {
				...(init?.headers ?? {}),
			},
		});

		if (!response.ok) {
			const text = await response.text();
			throw new Error(text || `API request failed (${response.status})`);
		}

		if (response.status === 204) {
			return undefined as T;
		}

		return (await response.json()) as T;
	}

	async listServers(): Promise<ServerListing[]> {
		return this.request<ServerListing[]>("/v1/servers", {
			method: "GET",
			headers: this.headers(),
		});
	}

	async upsertProfile(username?: string): Promise<void> {
		await this.request<{ ok: boolean }>("/v1/profile", {
			method: "POST",
			headers: this.headers(true),
			body: JSON.stringify({ username }),
		});
	}

	async getProfile(): Promise<PlayerProfile> {
		return this.request<PlayerProfile>("/v1/profile", {
			method: "GET",
			headers: this.headers(),
		});
	}

	async listFactions(): Promise<FactionSummary[]> {
		return this.request<FactionSummary[]>("/v1/factions", {
			method: "GET",
			headers: this.headers(),
		});
	}

	async createFaction(name: string): Promise<{ faction_id: number }> {
		return this.request<{ faction_id: number }>("/v1/factions", {
			method: "POST",
			headers: this.headers(true),
			body: JSON.stringify({ name }),
		});
	}

	async joinFaction(factionId: number): Promise<void> {
		await this.request<{ ok: boolean }>(`/v1/factions/${factionId}/join`, {
			method: "POST",
			headers: this.headers(),
		});
	}

	async leaveFaction(factionId: number): Promise<void> {
		await this.request<{ ok: boolean }>(`/v1/factions/${factionId}/leave`, {
			method: "POST",
			headers: this.headers(),
		});
	}

	async getLeaderboard(): Promise<LeaderboardEntry[]> {
		return this.request<LeaderboardEntry[]>("/v1/leaderboard/factions", {
			method: "GET",
			headers: this.headers(),
		});
	}
}
