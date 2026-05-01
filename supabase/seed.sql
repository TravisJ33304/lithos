insert into public.factions (name, wealth)
values ('Prospectors Guild', 0)
on conflict (name) do nothing;
