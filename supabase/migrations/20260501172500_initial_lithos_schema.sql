create extension if not exists pgcrypto;

create or replace function public.set_updated_at()
returns trigger
language plpgsql
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

create table if not exists public.factions (
  id bigserial primary key,
  name text not null unique,
  wealth bigint not null default 0,
  created_at timestamptz not null default now()
);

create table if not exists public.player_profiles (
  user_id text primary key,
  username text not null,
  faction_id bigint references public.factions(id) on delete set null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create trigger set_player_profiles_updated_at
before update on public.player_profiles
for each row
execute function public.set_updated_at();

create table if not exists public.faction_members (
  faction_id bigint not null references public.factions(id) on delete cascade,
  user_id text not null references public.player_profiles(user_id) on delete cascade,
  role text not null default 'member',
  primary key (faction_id, user_id)
);

create table if not exists public.progression (
  user_id text not null references public.player_profiles(user_id) on delete cascade,
  branch text not null,
  level integer not null default 1,
  xp integer not null default 0,
  xp_to_next integer not null default 100,
  primary key (user_id, branch)
);

create table if not exists public.overworld_wipes (
  id bigserial primary key,
  archive_label text not null,
  world_seed integer not null,
  archived_rows bigint not null,
  created_at timestamptz not null default now()
);

create table if not exists public.players (
  id uuid primary key default gen_random_uuid(),
  username varchar(255) not null,
  x double precision not null default 0,
  y double precision not null default 0,
  zone_id varchar(50) not null default 'overworld',
  health double precision not null default 100,
  inventory jsonb not null default '[]'::jsonb,
  auth_subject varchar(255),
  faction_id bigint references public.factions(id) on delete set null,
  last_login timestamptz default now()
);

create table if not exists public.base_structures (
  id bigserial primary key,
  zone_id varchar(50) not null,
  tile_type varchar(50) not null,
  grid_x integer not null,
  grid_y integer not null,
  unique (zone_id, grid_x, grid_y)
);

create index if not exists idx_players_username on public.players(username);
create index if not exists idx_players_auth_subject on public.players(auth_subject);
create index if not exists idx_player_profiles_faction_id on public.player_profiles(faction_id);
create index if not exists idx_faction_members_user_id on public.faction_members(user_id);
create index if not exists idx_progression_user_id on public.progression(user_id);
create index if not exists idx_base_structures_zone_id on public.base_structures(zone_id);

alter table public.factions enable row level security;
alter table public.player_profiles enable row level security;
alter table public.faction_members enable row level security;
alter table public.progression enable row level security;
alter table public.overworld_wipes enable row level security;
alter table public.players enable row level security;
alter table public.base_structures enable row level security;

drop policy if exists "factions are readable" on public.factions;
create policy "factions are readable"
on public.factions for select
to authenticated
using (true);

drop policy if exists "profiles are readable by authenticated users" on public.player_profiles;
create policy "profiles are readable by authenticated users"
on public.player_profiles for select
to authenticated
using (true);

drop policy if exists "users can upsert own profile" on public.player_profiles;
create policy "users can upsert own profile"
on public.player_profiles for insert
to authenticated
with check (user_id = auth.uid()::text);

drop policy if exists "users can update own profile" on public.player_profiles;
create policy "users can update own profile"
on public.player_profiles for update
to authenticated
using (user_id = auth.uid()::text)
with check (user_id = auth.uid()::text);

drop policy if exists "faction memberships are readable" on public.faction_members;
create policy "faction memberships are readable"
on public.faction_members for select
to authenticated
using (true);

drop policy if exists "users can read own progression" on public.progression;
create policy "users can read own progression"
on public.progression for select
to authenticated
using (user_id = auth.uid()::text);
