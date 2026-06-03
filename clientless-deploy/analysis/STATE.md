# Albion bot STATE — condensed tick10

GOAL: working Albion bot (navigate/gather/combat/bank/market/quest), driven via most-tractable surface per capability. metric = workstreams_done/5 (PARALLEL, not a serial spine). done = autonomous full loop (travel->gather/kill->return->bank/sell) no human input.

SCOPE (respected): op5-login-CLOSED — clientless from-scratch login is a TRUE external gate (EOS server-brokered, EAC-bound AES, capture-replay closed). DO NOT grind login. Behaviors are independent and advance NOW.

## Workstreams (1 worker + 1 box each)
- WS1 login/credential — box 39289025 — ACTIVE but OUT-OF-SCOPE this campaign (login gated/CLOSED). Account-creation path (not forging) is the only non-closed route; not on critical path now.
- WS2 nav / Move-codec byte-validation — box 39289029 — ACTIVE, metric 0/1. Toolchain DE-RISKED: selftest_validate.py proves validate_pcap Move hot-path (3/3 byte-exact). Only unknown = real wire layout. GATED on WS3 gameplay pcap.
- WS3 real-client on GPU box — box 39289031 (ssh1.vast.ai:19030, Titan Xp BusID 83:00.0) — ACTIVE, metric 0/1. **FRONTIER.** Client extracted (UNZIP_RC=0, 463 files), Xorg installed, binary chmod+x. /root/ws3_launch.sh has Xvfb :3 up + tcpdump armed (udp5056/5055 -> /root/ws3_gameplay.pcap) + Albion-Online RUNNING (PID4758, Unity booting). pcap 0 bytes (at boot/login). No creds on disk yet.
- WS4 gamestate ingest (quest/inv/bank/market) — box 39289034 — ACTIVE, offline 4/4, live 0/4. GATED on WS3 gameplay pcap.
- WS5 multi-account scaling — box 39292979 — PENDING, downstream of >=1 live-validated behavior.

## Frontier = WS3
blocker: client booting under Xvfb (softGL); needs to reach login screen, then a login cred to reach gameplay.
next: POLL ssh box31 'tail -30 /root/ws3_client.log; ls -la /root/ws3_gameplay.pcap; ps aux|grep -i albion'. If at login -> drive albion_ingame_register_login.py (DISPLAY=:3) with an account cred (mint via ~/albion/tools if none). If crashed -> read client.log for missing GL/lib, fall back Xorg :3 pinned to BusID 83:00.0 (GPU GLX). ONE gameplay pcap unblocks WS2+WS4 byte-validation simultaneously.

## Load-bearing facts
1. PRIOR-ART FIRST: ~/albion-wiki/WIKI.md + docs/account_creation_methods.md; ~/albion/tools/input/{navi-e2e/albion_e2e.py, albion_ingame_register_login.py}; tools/accounts/. Run these, don't re-derive. grep wiki/tools before building anything.
2. BOX ACCESS: ~/.ssh/id_ed25519 trusted by all pool boxes. ssh -i ~/.ssh/id_ed25519 -o StrictHostKeyChecking=accept-new -p <port> root@<host>. 5 boxes in pool.json. Image vastai/kvm:ubuntu_cli_22.04. Do NOT mint a new key.
3. Offline codec ALL GREEN (47 tests, 45 pass/2 skip): Connect/VerifyConnect byte-exact, PeerID 0x50FD, op22 seq+1, ACK piggyback, fragment reassembly. STOP polishing — gap is LIVE validation.
4. WS3 client: anon zip URL is the ONLY working download (200/3.09GB); direct/scrape/steam-anon CLOSED (CF403/no-resolve/no-sub). Extracted to /home/albion/albion-online on box31. tutorial-loop crate (733L) drove full tutorial on KVM previously.
5. captures/ = ONLY realm_login_051715.pcap (login, 0 op22 Moves). No gameplay pcap yet — WS3's /root/ws3_gameplay.pcap will be first. WS2 toolchain: pcap2tsv.py | validate_pcap.py /dev/stdin 5056.
6. C2S Move = op22; reach-steps server-inferred from position. Opcode tables on albion-wiki (762 rows); name_map regenerates per build — don't re-derive.

## WARNING (tick10): goal_tree.json was clobbered by a concurrent writer to a stale tick-0 serial S1-S4 version (contradicting episodes.log + real running client). Restored the authoritative WS tree. If it reverts again, episodes.log + this STATE.md are ground truth — a 2nd writer may be running ticks.
