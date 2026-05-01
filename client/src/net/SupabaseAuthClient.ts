import { createClient, type SupabaseClient } from "@supabase/supabase-js";

const PLACEHOLDER_VALUES = new Set([
	"",
	"your-anon-key-here",
	"https://your-project.supabase.co",
]);

export class SupabaseAuthClient {
	private readonly client: SupabaseClient | null;

	constructor() {
		const url = import.meta.env.VITE_SUPABASE_URL ?? "";
		const anonKey = import.meta.env.VITE_SUPABASE_ANON_KEY ?? "";
		this.client =
			PLACEHOLDER_VALUES.has(url) || PLACEHOLDER_VALUES.has(anonKey)
				? null
				: createClient(url, anonKey);
	}

	isConfigured(): boolean {
		return this.client !== null;
	}

	async signInOrSignUp(email: string, password: string): Promise<string> {
		if (!this.client) {
			throw new Error("Supabase auth is not configured");
		}
		const signIn = await this.client.auth.signInWithPassword({
			email,
			password,
		});
		if (signIn.data.session?.access_token) {
			return signIn.data.session.access_token;
		}
		const signUp = await this.client.auth.signUp({
			email,
			password,
		});
		const token = signUp.data.session?.access_token;
		if (!token) {
			throw signIn.error ?? signUp.error ?? new Error("Supabase auth failed");
		}
		return token;
	}
}
