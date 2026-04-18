## Overview
This codebase implements a single-threaded HTN controller that runs inside the proxy connection handler, derives a sensor snapshot from live packet state, selects the highest-priority applicable goal, decomposes that goal into a linear action stack, and then executes the stack incrementally while synchronizing injected movement/combat state back into `GameState`. The architecture is deliberately pragmatic rather than fully generic: most planning is static-priority method selection, most operators are packet emitters with cooldown/retry state stored in the process dictionary, and `GameState` plus CSV-backed tables provide the factual world, quest, item, and NPC data that make those decisions possible.

## Component Summary
- `Vampir.HTN`: Top-level HTN tick loop, goal selection, replanning, action execution, and status publishing.
- `Vampir.HTN.Planner`: Recursive HTN decomposer for Elixir methods and optional Tcl methods.
- `Vampir.HTN.ActionExecutor`: Runs the current action stack and applies per-goal backoff on failure.
- `Vampir.HTN.Action`: Shared action behavior, operator wrapping, and packet await helper.
- `Vampir.HTN.Method`: Behavior contract for HTN methods.
- `Vampir.HTN.Registry`: Discovers methods, caches goal metadata, and picks the next applicable goal.
- `Vampir.HTN.Sensors`: Converts `GameState` into planner-friendly facts and maintenance/combat/quest signals.
- `Vampir.HTN.TclBridge`: Safe Tcl adapter for external HTN methods; currently no live Tcl scripts are present.
- `Vampir.Proxy.Handler`: Per-connection event loop that feeds packets into HTN, blocks client packets, and schedules ticks.
- `Vampir.Inject`: Packet injection and wire-rewrite layer used by all HTN actions/operators.
- `Vampir.PacketLog`: ETS-backed decoded packet log used by `AwaitPkt` and debugging.
- `Vampir.HTN.CombatStats`: ETS combat event log for skills, damage, kills, and target churn.
- `Vampir.HTN.Methods.Pregame`: Char-select goal before the world is loaded.
- `Vampir.HTN.Methods.Quest`: Main story and quest progression method family.
- `Vampir.HTN.Methods.Combat`: Combat goal for aggro/self-defense and opportunistic quest combat.
- `Vampir.HTN.Methods.Resupply`: Potion resupply goal.
- `Vampir.HTN.Methods.Maintenance`: Auto-equip, upgrade, chest, collection, redraw, and periodic inventory goals.
- `Vampir.HTN.Methods.Survive`: Death recovery and instance exit on death.
- `Vampir.HTN.Methods.SoulAbsorb`: Goal for zombie/soul-absorb follow-up.
- `Vampir.HTN.Methods.SoulAbsorbNormal`: Goal for normal absorbable mobs with effect 23.
- `Vampir.HTN.Methods.WeepingVillage`: Hardcoded instance method for world `50010`.
- `Vampir.HTN.Methods.GoldFarm`: High-priority tutorial/farm method gated by `.enable_farm`.
- `Vampir.HTN.Actions.CharSelect`: Sends character select when pregame is active.
- `Vampir.HTN.Actions.Sleep`: Converts fixed sleeps into deadline-based waits.
- `Vampir.HTN.Actions.Wait`: Passive timed wait action.
- `Vampir.HTN.Actions.WaitUntil`: Polls world-state conditions until satisfied or timed out.
- `Vampir.HTN.Actions.AwaitPkt`: Polls `PacketLog` for an expected response packet.
- `Vampir.HTN.Actions.QuestStart`: Starts a quest and rejects non-zero `1804` results.
- `Vampir.HTN.Actions.StoryQuestStart`: Quest-start wrapper that optionally chains quest teleport for instance steps.
- `Vampir.HTN.Actions.QuestTeleport`: Sends `PktQuestTeleport`.
- `Vampir.HTN.Actions.QuestUpdate`: Sends `PktQuestUpdate` with rejection on non-zero `1811`.
- `Vampir.HTN.Actions.DungeonQuestUpdate`: Quest-update variant that tolerates dungeon step drift.
- `Vampir.HTN.Actions.StoryWalkUpdate`: Retry wrapper around `QuestUpdate` for waypoint advancement.
- `Vampir.HTN.Actions.StoryWaitStep`: Repeats quest updates for server-driven wait steps.
- `Vampir.HTN.Actions.StoryTutorial`: Resolves tutorial quest steps through tutorial packets or quest update fallback.
- `Vampir.HTN.Actions.TutorialUpdate`: Sends tutorial step progress.
- `Vampir.HTN.Actions.TutorialComplete`: Sends tutorial skip/complete.
- `Vampir.HTN.Actions.TalkTo`: NPC dialog/quest-update action for talk objectives.
- `Vampir.HTN.Actions.DialogMove`: Sends the synthetic dialog sequence move packet used before some quest updates.
- `Vampir.HTN.Actions.StoryInteract`: Orchestrates gadget/NPC interaction quest steps and fallback handling.
- `Vampir.HTN.Actions.GadgetInteract`: Resolves and interacts with gadgets for type-0 quest steps.
- `Vampir.HTN.Actions.QuestComplete`: Thin wrapper around the quest completion operator.
- `Vampir.HTN.Actions.QuestGiveUp`: Sends quest give-up.
- `Vampir.HTN.Actions.QuestInstantComplete`: Sends instant-complete for quest acts.
- `Vampir.HTN.Actions.QuestShopBuy`: Thin wrapper around the quest-shop operator buy path.
- `Vampir.HTN.Actions.QuestShopComplete`: Thin wrapper around the quest-shop operator complete path.
- `Vampir.HTN.Actions.DungeonEnter`: Sends dungeon entry.
- `Vampir.HTN.Actions.ExitDungeon`: Sends immediate world exit for expired/completed dungeons.
- `Vampir.HTN.Actions.WorldExit`: Sends world exit and awaits `215`.
- `Vampir.HTN.Actions.Revive`: Sends revive and awaits `319`.
- `Vampir.HTN.Actions.CombatTick`: Thin wrapper around the combat operator.
- `Vampir.HTN.Actions.KillOne`: Main target-focused combat executor.
- `Vampir.HTN.Actions.SoulAbsorb`: Uses the zombie absorb skill on corpse-state mobs.
- `Vampir.HTN.Actions.SoulAbsorbNormal`: Uses the normal absorb skill on effect-23 mobs.
- `Vampir.HTN.Actions.CircleKite`: Orbit-kiting combat action around an anchor point.
- `Vampir.HTN.Actions.BossKite`: Continuous boss-strafing combat action.
- `Vampir.HTN.Actions.MoveTo`: Path-following movement action with auto-mount and client sync.
- `Vampir.HTN.Actions.QuestMove`: Route planner for quest travel, lifts, and task teleports.
- `Vampir.HTN.Actions.LiftTravel`: Uses lift gadgets for cross-area travel.
- `Vampir.HTN.Actions.QuestGrind`: Patrol-and-fight loop for kill quests.
- `Vampir.HTN.Actions.BuyPotions`: Thin wrapper around the potion-buy operator.
- `Vampir.HTN.Actions.WalkToMerchant`: Thin wrapper around the merchant-walk operator.
- `Vampir.HTN.Actions.TeleportHome`: Thin wrapper around home teleport.
- `Vampir.HTN.Actions.BuySkillBooks`: Thin wrapper around the shop-book operator.
- `Vampir.HTN.Actions.UseSkillBook`: Thin wrapper around the skill-book use operator.
- `Vampir.HTN.Actions.UseSummonTicket`: Thin wrapper around the summon-ticket use operator.
- `Vampir.HTN.Actions.AutoEquip`: Chooses and equips better gear, costumes, and mounts.
- `Vampir.HTN.Actions.CollectionRegister`: Thin wrapper around collection registration.
- `Vampir.HTN.Actions.CombineCards`: Thin wrapper around card combination.
- `Vampir.HTN.Actions.CombineCardsBatch`: One-shot card combine batch executor.
- `Vampir.HTN.Actions.DecomposeJunk`: Thin wrapper around junk disassembly.
- `Vampir.HTN.Actions.EnhanceEquip`: Thin wrapper around safe enchant.
- `Vampir.HTN.Actions.RefineEquip`: Thin wrapper around refinement.
- `Vampir.HTN.Actions.OpenChests`: Thin wrapper around chest opening.
- `Vampir.HTN.Actions.RedrawCard`: Uses card redraw items on a timer.
- `Vampir.HTN.Actions.RedrawMount`: Uses mount redraw items on a timer.
- `Vampir.HTN.Actions.RefreshInventory`: Requests a full inventory refresh.
- `Vampir.HTN.Actions.ClaimAchievements`: Claims all completable achievements.
- `Vampir.HTN.Actions.GoldFarmTick`: Sends tutorial reward spam and item disassembly for the farm mode.
- `Vampir.HTN.Actions.TutorialCharacterCardDraw`: Tutorial packet spam for the costume draw tutorial.
- `Vampir.HTN.Actions.TutorialMountCardDraw`: Tutorial packet spam for the mount draw tutorial.
- `Vampir.HTN.Teleport`: Operator for `PktWorldMoveCast` type 2/13/21 teleports.
- `Vampir.HTN.Revive`: Operator-style revive loop with cooldown and time wall.
- `Vampir.HTN.QuestComplete`: Operator that completes quests based on quest state/progress.
- `Vampir.HTN.Combat`: Operator-style combat controller for quest areas and aggro.
- `Vampir.HTN.BuyPotions`: Operator that buys potions from the merchant.
- `Vampir.HTN.WalkToMerchant`: Operator that teleports home and pathfinds to the town merchant.
- `Vampir.HTN.BuySkillBooks`: Operator that opens the shop, buys the next class book, and checks the result packet.
- `Vampir.HTN.UseSkillBook`: Operator that uses inventory skill books.
- `Vampir.HTN.UseSummonTicket`: Operator that uses mount/costume/sephira draw tickets.
- `Vampir.HTN.CollectionRegister`: Operator that registers inventory items into collections.
- `Vampir.HTN.CombineCards`: Operator that batches card combinations by combination group.
- `Vampir.HTN.DecomposeJunk`: Operator that disassembles low-rarity junk gear.
- `Vampir.HTN.EnhanceEquipment`: Operator that safe-enchants equipped items.
- `Vampir.HTN.RefineEquipment`: Operator that refines equipped blue items with Trinity.
- `Vampir.HTN.OpenChests`: Operator that opens chest items when inventory has room.
- `Vampir.HTN.QuestShop`: Operator for purchase-guide quests.
- `Vampir.GameState`: Packet-driven world, quest, actor, inventory, skill, and collection state tracker.
- `Vampir.Tables.Quest`: CSV-backed quest/task/waypoint/action metadata loader.
- `Vampir.Tables.Tutorial`: CSV-backed tutorial step lookup.
- `Vampir.Tables.ClassTransform`: Dungeon transform-skill lookup for story instances.
- `Vampir.Tables.Skill`: Skill metadata, cooldown tracking, and skill picker.
- `Vampir.Tables.Shop`: Skill-book shop table.
- `Vampir.Tables.NpcSpawn`: NPC spawn positions by NPC info id and spawn id.
- `Vampir.Tables.Vehicle`: Vehicle metadata and movement speed lookup.
- `Vampir.Tables.Costume`: Costume metadata and rarity lookup.
- `Vampir.Tables.Combination`: Item template to combination-group lookup.
- `Vampir.Tables.Item`: Auctionability and collection-membership helper table.
- `Vampir.ItemData`: Item name/icon/rarity/equip/class/tier/safe-enchant metadata.
- `Vampir.NpcData`: NPC display/type/level/HP metadata and runtime NPC id cache.
- `Vampir.Pathfinder`: A* pathfinder with smoothing and component check.
- `Vampir.NavMesh`: Walkability grid, connected components, and boundary helpers.
- `Vampir.Portals`: Lift graph and BFS route finder for cross-area travel.

## Detailed Sections

### 1. HTN Planner / Decision Engine

#### `Vampir.HTN`
- File path: `lib/vampir/htn.ex`
- Purpose and behavior: Owns the per-connection HTN loop. It waits until the character has a level, world id, inventory, and local position; prunes stale aggro ids; derives sensors; picks the highest-priority goal; replans only when the current `action_stack` is empty; then executes via `ActionExecutor`.
- Key functions/callbacks: `do_tick/1`, `publish_status/2`, `tree_status/1`, `log_action/3`, `put_gs/1`, `get_gs/0`.
- Decision logic / priority / conditions: Goals come from `Registry.pick_goal/1`; the selected goal is decomposed once and then allowed to run to completion because the planner does not preempt a non-empty `action_stack`. If idle and no goal/action exists for 30 seconds, it injects a short random `MoveTo` to avoid idling.
- Packet interactions: No direct packet encoding. It indirectly causes packets through pushed actions and synchronizes injected movement/combat fields back into handler state.
- Thresholds, constants, config values: Waits for level from stat `3`; anti-idle cadence `30_000 ms`; random anti-idle offset up to `3.0` units; status cache ETS key `:vampir_bt_status`.
- How it interacts with other components: Uses `Sensors`, `Registry`, `Planner`, `ActionExecutor`, and `GameState`; runs inside `Proxy.Handler`; syncs `local_pos`, `skill_cds`, `last_skill_id`, and `last_normal_id` from `Process.get(:htn_gs)`.

#### `Vampir.HTN.Planner`
- File path: `lib/vampir/htn/planner.ex`
- Purpose and behavior: Recursively decomposes a goal into a flat step list.
- Key functions/callbacks: `plan/3`, `decompose/5`.
- Decision logic / priority / conditions: Supports both Elixir method modules and Tcl routes. Decomposition depth is capped to avoid runaway recursion; failure returns `:no_plan`.
- Packet interactions: None.
- Thresholds, constants, config values: `@max_decompose_depth = 10`.
- How it interacts with other components: Reads routes from `Registry`; calls method callbacks or `TclBridge.decompose/4`.

#### `Vampir.HTN.ActionExecutor`
- File path: `lib/vampir/htn/action_executor.ex`
- Purpose and behavior: Executes the current plan/action stack one step at a time.
- Key functions/callbacks: `tick/1`, `push/2`, `idle?/1`, `goal_backed_off?/1`.
- Decision logic / priority / conditions: Uses the head of `action_stack` as the only active task. On action failure it clears `action_stack`, `plan`, and `goal`, and applies a per-goal backoff so the same goal will not be immediately reselected.
- Packet interactions: None directly; delegates to action modules.
- Thresholds, constants, config values: Goal backoff `10_000 ms`; does not log noise actions `Sleep`, `Wait`, `AwaitPkt`, and `WaitUntil`.
- How it interacts with other components: Called from `Vampir.HTN`; dispatches through `Vampir.HTN.Action`; `Registry.pick_goal/1` respects the executor backoff map.

#### `Vampir.HTN.Action`
- File path: `lib/vampir/htn/action.ex`
- Purpose and behavior: Base behavior for HTN actions and shared helpers for packet injection plus operator wrapping.
- Key functions/callbacks: protocol callbacks `step/2`, `describe/1`; helpers `inject_and_await/6`, `run_operator/4`.
- Decision logic / priority / conditions: `run_operator/4` executes operator logic on an isolated sub-stack and merges any operator-pushed actions back into the parent state.
- Packet interactions: `inject_and_await/6` records the current `PacketLog.last_seq/0`, injects a packet via `Vampir.Inject.send/2`, then returns an `AwaitPkt` action watching a specific response opcode.
- Thresholds, constants, config values: Inherits timeout and match/reject functions from each caller.
- How it interacts with other components: Used by every action module; depends on `AwaitPkt`, `PacketLog`, and `Inject`.

#### `Vampir.HTN.Method`
- File path: `lib/vampir/htn/method.ex`
- Purpose and behavior: Behavior contract for method modules.
- Key functions/callbacks: `__goals__/0`, `applicable?/2`, `decompose/3`.
- Decision logic / priority / conditions: None itself; it defines the planner interface.
- Packet interactions: None.
- Thresholds, constants, config values: None.
- How it interacts with other components: Implemented by all `Methods.*` modules and consumed by `Registry` and `Planner`.

#### `Vampir.HTN.Registry`
- File path: `lib/vampir/htn/registry.ex`
- Purpose and behavior: Discovers all method modules and optional Tcl methods, caches goal metadata, and picks the next goal.
- Key functions/callbacks: `methods/0`, `goals/0`, `route/1`, `pick_goal/1`.
- Decision logic / priority / conditions: Goals are sorted by descending `priority`, then filtered by `applicable?/2`. `pick_goal/1` also skips goals currently backed off by `ActionExecutor`.
- Packet interactions: None.
- Thresholds, constants, config values: Uses `persistent_term` caches for discovered methods/goals/routes.
- How it interacts with other components: Feeds `Vampir.HTN` and `Planner`; delegates Tcl discovery to `TclBridge`.

#### `Vampir.HTN.Sensors`
- File path: `lib/vampir/htn/sensors.ex`
- Purpose and behavior: Builds the planner input snapshot from `GameState`.
- Key functions/callbacks: `read/2` and a set of private quest/combat/maintenance helper predicates.
- Decision logic / priority / conditions: Derives `low_potions`, `near_merchant`, `in_quest_area`, `active_kill_quest`, `target_waypoint`, `completable_quest`, `shop_quest`, `next_main_quest`, `has_upgrade`, `needs_enhance`, `needs_refine`, `has_skill_books`, `has_summon_tickets`, `has_chests`, `achievement_claim_due`, `inventory_refresh_due`, `in_instance`, `instance_has_quest`, and `dungeon_expired`.
- Packet interactions: None directly; all inputs come from `GameState`.
- Thresholds, constants, config values: HP stat `4`; level stat `3`; heal threshold `0.6`; potion floor `50`; potion template `91_100_005`; merchant range `500.0`; merchant NPC ids `[203, 10130, 10231, 91031, 91033, 10610, 10616, 10617, 10618]`; min buy level `10`; inventory refresh every `60_000 ms`; achievements every `120_000 ms`; combine cadence every `4_000 ms`; quest completion attempts limited to `3`.
- How it interacts with other components: Uses `Methods.Quest.next_waypoint/1`, `Methods.Quest.in_instance?/1`, `HTN.Combat`, `Actions.AutoEquip`, `EnhanceEquipment`, `RefineEquipment`, `BuySkillBooks`, `UseSkillBook`, `UseSummonTicket`, `CombineCards`, `Tables.Quest`, and process-dictionary retry state.

#### `Vampir.HTN.TclBridge`
- File path: `lib/vampir/htn/tcl_bridge.ex`
- Purpose and behavior: Lets the planner source goals and decompositions from Tcl scripts in `priv/htn_scripts`.
- Key functions/callbacks: `discover/0`, `applicable?/3`, `decompose/4`.
- Decision logic / priority / conditions: Converts the sensor map into a flat Tcl dict and expects Tcl procedures `__goals__`, `applicable?`, and `decompose`. In the current repo there are no actual `.tcl` HTN scripts, only `priv/htn_scripts/.gitkeep`.
- Packet interactions: None.
- Thresholds, constants, config values: Safe Tcl interpreter only registers variables/control/procedures/strings/lists/dicts/expr commands.
- How it interacts with other components: Used by `Registry` and `Planner` as an alternate method route.

#### `Vampir.Proxy.Handler`
- File path: `lib/vampir/proxy/handler.ex`
- Purpose and behavior: Per-connection GenServer that relays packets, updates game state, and drives HTN ticks.
- Key functions/callbacks: `handle_info/2` branches for C->S, S->C, `:htn_enable`, `:htn_disable`, `:htn_tick`, `:htn_walk_to`, `:htn_clear_blocks`, and inject/block commands.
- Decision logic / priority / conditions: Detects game streams, auto-enables HTN after `5_000 ms`, and schedules HTN ticks at `gs.move_interval` or `200 ms`. On HTN crash it records a crash report and backs the next tick off to `5_000 ms`. Even when HTN is disabled it still drains queued actions and publishes sensors once per second.
- Packet interactions: Blocks client opcodes `305`, `607`, `1803`, `1810`, `1820`, and `9903` when HTN is enabled; also blocks client gadget interact on enable. It defines an S->C quest block list, but the automatic enable path has the S->C block union commented out, so that list is not currently enforced by default.
- Thresholds, constants, config values: `@htn_tick_ms = 200`; tutorial intercept defaults `%{4008 => 10, 4009 => 12}`; headless mode whitelists `8416` and `16`.
- How it interacts with other components: Publishes packets into `GameState` before filtering, calls `Vampir.HTN.do_tick/1`, syncs `:htn_gs` back into handler `gs`, uses `Inject` for rewrites and injection, and clears process-dictionary retry/block keys via `:htn_clear_blocks`.

#### `Vampir.Inject`
- File path: `lib/vampir/inject.ex`
- Purpose and behavior: Centralized wire-level packet injector and rewriter.
- Key functions/callbacks: `send/2`, `send_batch/1`, `send_via_handler/2`, `send_queued/2`, `send_s2c/2`, `rewrite_c2s/4`, `filter_s2c/2`, `teleport_client/3`, `lift_client/3`, `set_206_template/1`, `set_355_template/1`.
- Decision logic / priority / conditions: Rewrites client packet sequence numbers, optionally filters blocked/whitelisted opcodes, and can patch S->C stat packets for level spoofing.
- Packet interactions: Encodes and injects all HTN C->S traffic; can also inject S->C `PktWorldMoveFinishResult` and `PktLiftNotify` to visually sync the client.
- Thresholds, constants, config values: Quiet inject opcode set includes movement/ping; tutorial skip payload is rewritten to force `Complete=false`.
- How it interacts with other components: Used by all actions/operators; `GameState` caches raw `206`/`355` templates for `teleport_client/3` and `lift_client/3`; `Proxy.Handler` uses the rewrite/filter helpers.

#### `Vampir.PacketLog`
- File path: `lib/vampir/packet_log.ex`
- Purpose and behavior: ETS-backed decoded packet log used for debugging and HTN packet waits.
- Key functions/callbacks: `log/3`, `log_injected/3`, `recent/1`, `last_seq/0`, `filtered/1`, `clear/0`.
- Decision logic / priority / conditions: None for planning, but `AwaitPkt` depends on monotonically increasing ETS keys and direction/opcode matching.
- Packet interactions: Stores every decoded packet and injected packet, including payload bytes and decode status.
- Thresholds, constants, config values: Maximum `100_000` entries; noise opcode filter includes movement, sight, effects, and other high-volume packets.
- How it interacts with other components: `Action.inject_and_await/6` records `last_seq/0`; `AwaitPkt` scans forward from that sequence; `BuySkillBooks` inspects recent purchase results.

#### `Vampir.HTN.Actions.Sleep`
- File path: `lib/vampir/htn/actions/sleep.ex`
- Purpose and behavior: Convenience wrapper that converts a fixed delay into a `Wait` action on first tick.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: No local decisions.
- Packet interactions: None.
- Thresholds, constants, config values: Uses the caller-provided `ms`.
- How it interacts with other components: Used throughout method decompositions as an explicit pacing primitive.

#### `Vampir.HTN.Actions.Wait`
- File path: `lib/vampir/htn/actions/wait.ex`
- Purpose and behavior: Passive deadline-based wait.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: Initializes `until` once and reports `:done` when the deadline passes.
- Packet interactions: None.
- Thresholds, constants, config values: Uses the caller-provided `ms` or `until`.
- How it interacts with other components: Used by `Sleep` and directly by some method decompositions.

#### `Vampir.HTN.Actions.WaitUntil`
- File path: `lib/vampir/htn/actions/wait_until.ex`
- Purpose and behavior: Polls state-derived conditions until they become true or a timeout expires.
- Key functions/callbacks: `step/2`, private `check_condition/3`.
- Decision logic / priority / conditions: Supports `:near_pos`, `:walk_done`, `:not_dead`, `:has_pos`, and `:world_changed`.
- Packet interactions: None directly; it reacts to packet-driven `GameState`.
- Thresholds, constants, config values: Timeout is per action; failure message is `wait_until:<cond> timeout`.
- How it interacts with other components: Commonly used after teleports, char select, or injected movement.

#### `Vampir.HTN.Actions.AwaitPkt`
- File path: `lib/vampir/htn/actions/await_pkt.ex`
- Purpose and behavior: Polls `PacketLog` for a specific response packet after injection.
- Key functions/callbacks: `new/5`, `step/2`.
- Decision logic / priority / conditions: Starts scanning at `since_seq`; only matches packets with the expected opcode and direction; optional `match` and `reject` callbacks refine success/failure.
- Packet interactions: Watches injected request/response flows such as quest start/update/teleport, dungeon enter, revive, and world exit.
- Thresholds, constants, config values: Per-call timeout; default direction `"S->C"`.
- How it interacts with other components: Created by `Action.inject_and_await/6`; depends on `PacketLog`.

### 2. Quest Progression

#### `Vampir.HTN.Methods.Pregame`
- File path: `lib/vampir/htn/methods/pregame.ex`
- Purpose and behavior: Handles the character-select phase before the world is entered.
- Key functions/callbacks: `__goals__/0`, `applicable?/2`, `decompose/3`.
- Decision logic / priority / conditions: Goal `:char_select` priority `100`; applicable when connected but not fully in world yet.
- Packet interactions: Decomposes to `CharSelect`, `Sleep 5000`, `WaitUntil has_pos`.
- Thresholds, constants, config values: Waits up to `30_000 ms` for position after selection.
- How it interacts with other components: Runs before all combat/quest logic; depends on `Actions.CharSelect`.

#### `Vampir.HTN.Methods.Quest`
- File path: `lib/vampir/htn/methods/quest.ex`
- Purpose and behavior: Main story/quest method family for accepting quests, moving to objectives, killing targets, interacting with NPCs/gadgets, teleporting into instances, and completing quests.
- Key functions/callbacks: `__goals__/0`, `applicable?/2`, `decompose/3`, `next_waypoint/1`, `in_instance?/1`, `story_character?/1`, helper NPC/gadget/step functions.
- Decision logic / priority / conditions: Goals and priorities are `:exit_expired_dungeon` `95`, `:quest_complete` `75`, `:quest_accept_next` `48`, `:quest_shop` `45`, `:quest_story` `10`. `next_waypoint/1` prioritizes dungeon quests, same-navmesh-component steps while in instances, and ignores UI-only waypoint types `22/23/24/25/27`. It also synthesizes a finalization waypoint when a story quest is active with `progress_target == 0` and all waypoints are done.
- Packet interactions: Decomposition can produce `QuestStart`, `QuestTeleport`, `QuestUpdate`, `DungeonQuestUpdate`, `DungeonEnter`, `ExitDungeon`, `QuestComplete`, `TeleportHome`, `WalkToMerchant`, `TalkTo`, `DialogMove`, and `StoryInteract`.
- Thresholds, constants, config values: Blocks a stuck task for `5 minutes` via `{:update_blocked_until, task_id}`; `dungeon_id_from_gadgets/1` uses `div(config_id, 1000)` for gadget config ids `>= 80000`.
- How it interacts with other components: It is the central consumer of `Tables.Quest`, `Actions.QuestMove`, `Actions.QuestGrind`, `Actions.StoryWalkUpdate`, `Actions.StoryInteract`, `Actions.TalkTo`, `Actions.GadgetInteract`, and instance helpers.

#### `Vampir.HTN.Actions.CharSelect`
- File path: `lib/vampir/htn/actions/char_select.ex`
- Purpose and behavior: Sends a character select request using `gs.last_played_char_id`.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: Returns done if already in world with a position; otherwise enforces a local resend cooldown.
- Packet interactions: Sends `PktCharacterSelect` with payload `<<char_id::le64, 0::le32>>`.
- Thresholds, constants, config values: Cooldown `15_000 ms`.
- How it interacts with other components: Used only by `Methods.Pregame`; relies on `Sensors.read/2`.

#### `Vampir.HTN.Actions.QuestStart`
- File path: `lib/vampir/htn/actions/quest_start.ex`
- Purpose and behavior: Starts a quest and awaits the start result.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: Rejects any `1804` response whose first result field is non-zero.
- Packet interactions: Sends `PktQuestStart`; awaits `PktQuestStartResult` (`1804`).
- Thresholds, constants, config values: Timeout `5_000 ms`.
- How it interacts with other components: Used by `Methods.Quest`, `StoryQuestStart`, and `QuestShop`.

#### `Vampir.HTN.Actions.StoryQuestStart`
- File path: `lib/vampir/htn/actions/story_quest_start.ex`
- Purpose and behavior: Story-aware wrapper around quest start that optionally teleports into the correct instance first.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: If the quest is already active it returns done. If the first waypoint requires an instance and `zone_quest_id` differs, it pushes `QuestStart`, `QuestTeleport`, `Wait 3000`; otherwise it pushes only `QuestStart`. It also avoids sending immediately after instance entry.
- Packet interactions: Delegates to `QuestStart` and `QuestTeleport`.
- Thresholds, constants, config values: Waits until at least `1_000 ms` has passed after `:instance_entry_ts`.
- How it interacts with other components: Used by `Methods.Quest` for accepted-but-not-started story quests.

#### `Vampir.HTN.Actions.QuestTeleport`
- File path: `lib/vampir/htn/actions/quest_teleport.ex`
- Purpose and behavior: Sends quest-specific teleport requests.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: No local conditions.
- Packet interactions: Sends `PktQuestTeleport` payload `<<type::8, quest_id::le64, 1::8>>`; awaits `PktQuestTeleportResult` (`1821`).
- Thresholds, constants, config values: Timeout `10_000 ms`; default `type` is `0`, but `Methods.Quest` uses `5` for instance teleports.
- How it interacts with other components: Used by `Methods.Quest`, `StoryQuestStart`, and instance quest flows.

#### `Vampir.HTN.Actions.QuestUpdate`
- File path: `lib/vampir/htn/actions/quest_update.ex`
- Purpose and behavior: Sends a quest progress/update packet using the current or supplied position.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: Rejects non-zero `1811` responses, which is how movement/interact/story logic notices “too far” and other server rejections.
- Packet interactions: Sends `PktQuestUpdate`; awaits `PktQuestUpdateResult` (`1811`).
- Thresholds, constants, config values: Timeout `5_000 ms`.
- How it interacts with other components: Used by `StoryWalkUpdate`, `StoryWaitStep`, `StoryInteract`, `TalkTo`, and some dungeon methods.

#### `Vampir.HTN.Actions.DungeonQuestUpdate`
- File path: `lib/vampir/htn/actions/dungeon_quest_update.ex`
- Purpose and behavior: Dungeon-specific quest-update wrapper.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: Accepts drift between client-side and server-side dungeon step indices by not treating `1007` as fatal.
- Packet interactions: Sends `PktQuestUpdate`; awaits `1811`.
- Thresholds, constants, config values: Timeout `5_000 ms`.
- How it interacts with other components: Used by `Methods.WeepingVillage`.

#### `Vampir.HTN.Actions.StoryWalkUpdate`
- File path: `lib/vampir/htn/actions/story_walk_update.ex`
- Purpose and behavior: Retry wrapper for waypoint-completion quest updates.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: If the current step already advanced it returns done. Otherwise it increments `{:update_attempts, task_id}` and fails once attempts exceed `5`.
- Packet interactions: Pushes a `QuestUpdate` action using the waypoint/task index and position.
- Thresholds, constants, config values: Max attempts `5`.
- How it interacts with other components: Used by `Methods.Quest` and as fallback in `StoryTutorial`.

#### `Vampir.HTN.Actions.StoryWaitStep`
- File path: `lib/vampir/htn/actions/story_wait_step.ex`
- Purpose and behavior: Handles server-driven “wait” quest steps where the client must keep poking `QuestUpdate`.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: Returns done when the quest step advances or the quest becomes completable; otherwise resends updates on a timer until timeout.
- Packet interactions: Repeatedly sends `PktQuestUpdate`.
- Thresholds, constants, config values: Retry interval `5_000 ms`; overall timeout `90_000 ms`.
- How it interacts with other components: Used by `Methods.Quest` for type-5 wait steps.

#### `Vampir.HTN.Actions.StoryTutorial`
- File path: `lib/vampir/htn/actions/story_tutorial.ex`
- Purpose and behavior: Resolves tutorial quest steps.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: Uses `gs.pending_tutorial` or waypoint `tutorial_id`; if no tutorial is pending it falls back to `StoryWalkUpdate`.
- Packet interactions: Pushes `TutorialUpdate` followed by `TutorialComplete`, or falls back to `QuestUpdate`.
- Thresholds, constants, config values: Tutorial server step comes from `Tables.Tutorial.server_step/1`, defaulting to `2`.
- How it interacts with other components: Used by `Methods.Quest`; depends on `Tables.Tutorial`.

#### `Vampir.HTN.Actions.TutorialUpdate`
- File path: `lib/vampir/htn/actions/tutorial_update.ex`
- Purpose and behavior: Sends a tutorial step completion packet.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None.
- Packet interactions: Sends `PktTutorialUpdate` payload `<<info_id::le32, step::le32, 1::8>>`; awaits `9904`.
- Thresholds, constants, config values: Timeout `5_000 ms`.
- How it interacts with other components: Used by `StoryTutorial` and the gold-farm tutorial actions.

#### `Vampir.HTN.Actions.TutorialComplete`
- File path: `lib/vampir/htn/actions/tutorial_complete.ex`
- Purpose and behavior: Completes/skips a tutorial.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None.
- Packet interactions: Sends `PktTutorialSkip` payload `<<info_id::le32, 1::8>>`.
- Thresholds, constants, config values: None.
- How it interacts with other components: Used by `StoryTutorial`.

#### `Vampir.HTN.Actions.TalkTo`
- File path: `lib/vampir/htn/actions/talk_to.ex`
- Purpose and behavior: Resolves an NPC by quest spawn info id, then attempts the talk/update sequence.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: Tries up to `5` candidate NPCs, preferring the most recently seen actors. If the NPC is not found it retries a bounded number of times. The current implementation advances the candidate index immediately when the injected request returns `{:cont, ...}`, which is a porting caution because that makes the flow slightly odd.
- Packet interactions: Sends `PktQuestUpdate` with the NPC entity id encoded as `id_param`.
- Thresholds, constants, config values: Candidate limit `5`; bounded wait loop before failure.
- How it interacts with other components: Used by `Methods.Quest` and `StoryInteract`; depends on `NpcSpawn` and live actor state.

#### `Vampir.HTN.Actions.DialogMove`
- File path: `lib/vampir/htn/actions/dialog_move.ex`
- Purpose and behavior: Sends the dialog-sequence packet that some story interactions expect before the quest update.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: Computes a synthetic dialog sequence from quest id and task id.
- Packet interactions: Sends two `PktDialogMove` packets, one with `sync=1` and one with `sync=0`; awaits `1853` and rejects non-zero results.
- Thresholds, constants, config values: `dialog_seq = quest_id * 10_000_000 + rem(task_id, 1000) * 10_000 + 2001`.
- How it interacts with other components: Used by `StoryInteract` and some NPC quest steps.

#### `Vampir.HTN.Actions.StoryInteract`
- File path: `lib/vampir/htn/actions/story_interact.ex`
- Purpose and behavior: Handles type-0 interact steps against gadgets or NPCs.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: If the step already advanced it returns done. If gadget config ids exist it pushes `GadgetInteract`, `Sleep 10000`, and `QuestUpdate`. Otherwise it looks for the nearest matching NPC and pushes `DialogMove` plus `QuestUpdate`. If no target appears it increments `{:no_target_waits, task_id}` and eventually falls back to a raw `QuestUpdate`; after `3` such fallbacks it blocks the task for `5 minutes`.
- Packet interactions: Delegates to gadget interaction, dialog move, and quest update packets.
- Thresholds, constants, config values: Max missing-target waits `60` in instances and `15` outside; fallback block `5 minutes`.
- How it interacts with other components: Central interaction orchestrator for `Methods.Quest`.

#### `Vampir.HTN.Actions.GadgetInteract`
- File path: `lib/vampir/htn/actions/gadget_interact.ex`
- Purpose and behavior: Resolves quest gadgets from waypoint config ids and interacts with them.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: If a matching gadget is visible it may first push `QuestMove` to the gadget. If no gadget is visible but a spawn position is known, it walks toward the spawn; otherwise it fails.
- Packet interactions: Sends `PktGadgetControlStart`; awaits `334` and rejects non-zero responses such as `1105` (“too far”).
- Thresholds, constants, config values: Moves toward spawn if farther than roughly `500` units.
- How it interacts with other components: Used by `StoryInteract`; depends on `Tables.Quest.gadget_spawn_pos/3` and live gadget state.

#### `Vampir.HTN.Actions.QuestComplete`
- File path: `lib/vampir/htn/actions/quest_complete.ex`
- Purpose and behavior: Thin action wrapper around `Vampir.HTN.QuestComplete`.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: No local logic; inherits operator checks and retries.
- Packet interactions: None directly; operator sends `PktQuestComplete`.
- Thresholds, constants, config values: None local.
- How it interacts with other components: Used by `Methods.Quest`.

#### `Vampir.HTN.QuestComplete`
- File path: `lib/vampir/htn/operators/quest_complete.ex`
- Purpose and behavior: Decides which quest can be completed and sends the complete request.
- Key functions/callbacks: `check/1`, `run/1`.
- Decision logic / priority / conditions: Completes quests whose status is `:completable`, whose `progress >= progress_target`, or whose story waypoints are exhausted with `progress_target == 0`. It tracks `:quest_complete_attempts` and normally caps attempts at `3`; story-finalization fallback is capped at `1`.
- Packet interactions: Sends `PktQuestComplete` payload `<<1::le16, quest_id::le64, 0::8>>`.
- Thresholds, constants, config values: Cooldown `5_000 ms`; attempt cap `3` or `1` depending on case.
- How it interacts with other components: Used by `Methods.Quest` and `Sensors.completable_quest`; `GameState` consumes `1816` and reward packets after completion.

#### `Vampir.HTN.Actions.QuestGiveUp`
- File path: `lib/vampir/htn/actions/quest_give_up.ex`
- Purpose and behavior: Gives up a quest.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None.
- Packet interactions: Sends `PktQuestGiveUp`; awaits `1846`.
- Thresholds, constants, config values: Timeout `5_000 ms`.
- How it interacts with other components: Available for recovery/manual flows; not a mainline method output today.

#### `Vampir.HTN.Actions.QuestInstantComplete`
- File path: `lib/vampir/htn/actions/quest_instant_complete.ex`
- Purpose and behavior: Instant-completes a quest act.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None.
- Packet interactions: Sends `PktQuestActInstantComplete`; awaits `1830`.
- Thresholds, constants, config values: Timeout `5_000 ms`.
- How it interacts with other components: Utility action for specialized quest debugging/automation.

#### `Vampir.HTN.Actions.QuestShopBuy`
- File path: `lib/vampir/htn/actions/quest_shop_buy.ex`
- Purpose and behavior: Thin wrapper around the quest-shop buy operator.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None local.
- Packet interactions: None directly; operator sends quest start or shop purchase.
- Thresholds, constants, config values: None local.
- How it interacts with other components: Used by `Methods.Quest` for purchase-guide quests.

#### `Vampir.HTN.Actions.QuestShopComplete`
- File path: `lib/vampir/htn/actions/quest_shop_complete.ex`
- Purpose and behavior: Thin wrapper around the quest-shop complete operator.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None local.
- Packet interactions: None directly; operator sends `PktQuestComplete` when ready.
- Thresholds, constants, config values: None local.
- How it interacts with other components: Used by `Methods.Quest`.

#### `Vampir.HTN.QuestShop`
- File path: `lib/vampir/htn/operators/quest_shop.ex`
- Purpose and behavior: Handles merchant guide quests that require buying one item.
- Key functions/callbacks: `buy/1`, `complete/1`.
- Decision logic / priority / conditions: `buy/1` looks for quest type `7` quests with no targets/waypoints and objective text containing “purchase”. If the quest is still `:accepted` it sends `QuestStart`; otherwise it buys one product. `complete/1` completes once the shop quest becomes completable or reaches target progress.
- Packet interactions: Sends `PktQuestStart`, `PktShopItemPurchase`, and `PktQuestComplete`.
- Thresholds, constants, config values: Uses potion product `10_500_003` and shop type `5`.
- How it interacts with other components: Wrapped by `QuestShopBuy` and `QuestShopComplete`; selected through `Methods.Quest`.

#### `Vampir.HTN.Actions.DungeonEnter`
- File path: `lib/vampir/htn/actions/dungeon_enter.ex`
- Purpose and behavior: Enters a dungeon from a quest interaction step.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None.
- Packet interactions: Sends `PktDungeonEnter`; awaits `4004`.
- Thresholds, constants, config values: Timeout `10_000 ms`.
- How it interacts with other components: Used by `Methods.Quest` when a gadget config id implies a dungeon id.

#### `Vampir.HTN.Actions.ExitDungeon`
- File path: `lib/vampir/htn/actions/exit_dungeon.ex`
- Purpose and behavior: Immediate dungeon/world exit action.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None; it fires once.
- Packet interactions: Sends `PktWorldExit` (`214`) without waiting.
- Thresholds, constants, config values: None.
- How it interacts with other components: Used by `Methods.Quest` for expired dungeons and by `Methods.WeepingVillage` after completion.

#### `Vampir.HTN.Actions.WorldExit`
- File path: `lib/vampir/htn/actions/world_exit.ex`
- Purpose and behavior: World/instance exit action with response wait.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None.
- Packet interactions: Sends `PktWorldExit` (`214`); awaits `PktWorldExitResult` (`215`).
- Thresholds, constants, config values: Timeout `10_000 ms`.
- How it interacts with other components: Used by `Methods.Survive` when dead inside an instance.

#### `Vampir.Tables.Quest`
- File path: `lib/vampir/tables/quest.ex`
- Purpose and behavior: Loads quest metadata, task metadata, sequence names, task waypoints, task actions, kill targets, and gadget spawn positions from multiple CSV files.
- Key functions/callbacks: `get/1`, `name/1`, `task/1`, `sequence_name/1`, `waypoints/1`, `target_npc_ids/1`, `required_quest/1`, `main_quest_blocker/1`, `task_actions/1`, `gadget_spawn_pos/3`.
- Decision logic / priority / conditions: None at runtime beyond data lookup, but the waypoint loader merges JSON `Param` fields into the waypoint map so planner logic can see `npc_spawn_id`, `tutorial_id`, `dialog_group_id`, `dest_pos`, `npc_info_ids`, `gadget_config_ids`, `reward_list`, and more.
- Packet interactions: None.
- Thresholds, constants, config values: Reads `Quest.csv`, `QuestTask.csv`, `QuestTaskAction.csv`, `QuestSequence_Name.csv`, `QuestTask_Name.csv`, `Quest_Name.csv`, and `GadgetSpawn.csv`.
- How it interacts with other components: Core data source for `Methods.Quest`, `HTN.Combat`, `QuestGrind`, `TalkTo`, `GadgetInteract`, `Sensors`, and `GameState` reward/blocker helpers.

#### `Vampir.Tables.Tutorial`
- File path: `lib/vampir/tables/tutorial.ex`
- Purpose and behavior: Loads tutorial step metadata and exposes the first server-driven step per tutorial.
- Key functions/callbacks: `server_step/1`.
- Decision logic / priority / conditions: Keeps the lowest `ServerProcessRequest=1` step for each tutorial id.
- Packet interactions: None.
- Thresholds, constants, config values: Reads `TutorialStep.csv`.
- How it interacts with other components: Used by `Actions.StoryTutorial`.

#### `Vampir.Tables.ClassTransform`
- File path: `lib/vampir/tables/class_transform.ex`
- Purpose and behavior: Maps dungeon ids and class ids to transform skill lists for story-character instances.
- Key functions/callbacks: `skills_for/2`.
- Decision logic / priority / conditions: Extracts `ClassTransform` data from `Dungeon.csv` JSON and `EquipSkillList` from `ClassTransform.csv`.
- Packet interactions: None.
- Thresholds, constants, config values: Reads `Dungeon.csv` and `ClassTransform.csv`.
- How it interacts with other components: `GameState` calls it after `PktWorldMoveFinishResult` to swap in transform skills in story dungeons.

### 3. Combat

#### `Vampir.HTN.Methods.Combat`
- File path: `lib/vampir/htn/methods/combat.ex`
- Purpose and behavior: Exposes the planner goal for direct combat when the player has aggro or nearby combat targets.
- Key functions/callbacks: `__goals__/0`, `applicable?/2`, `decompose/3`.
- Decision logic / priority / conditions: Goal `:combat` priority `70`; applicable when alive and either `has_aggro` or `has_combat_targets`. It selects the target by preferring aggro NPCs, then quest targets, then nearest valid mob, and decomposes to `KillOne`.
- Packet interactions: No direct packets; it decomposes into combat actions.
- Thresholds, constants, config values: Uses a local `3000` unit scan for nearby targets.
- How it interacts with other components: Consumes `Sensors`; uses `Tables.Quest.target_npc_ids/1`; delegates to `Actions.KillOne`.

#### `Vampir.HTN.Combat`
- File path: `lib/vampir/htn/operators/combat.ex`
- Purpose and behavior: Operator-style combat controller used by `CombatTick` and quest grinding.
- Key functions/callbacks: `check/1`, `run/1`, `active_kill_quest/1`, `in_quest_area?/1`, `quest_combat_area/1`.
- Decision logic / priority / conditions: Combat is active if the player is alive and either has aggro or is inside the area of an active kill quest. It refuses to enter combat below the heal threshold unless the mob is already aggroed. It prefers sticky targets, blacklists invalid targets after some failure responses, and yields if a non-combat quest waypoint is pending so movement can continue.
- Packet interactions: Sends `PktSupportModeStopRequest` once to disable support mode and blocks the client from re-sending it; sends `PktSkillStart` for attacks; reads `608` skill results via `GameState.drain_skill_results/1`.
- Thresholds, constants, config values: Attack range `400.0`; attack cooldown `750 ms`; heal threshold `0.6`; target blacklist `30_000 ms`; quest-area radius `5000.0`; HP stat `4`.
- How it interacts with other components: Uses `Tables.Skill`, `Tables.Quest`, `Pathfinder`, `MoveTo`, `GameState`, and process-dictionary state such as `:current_target`, `:pending_attacks`, `:target_blacklist`, and `:combat_since`.

#### `Vampir.HTN.Actions.CombatTick`
- File path: `lib/vampir/htn/actions/combat_tick.ex`
- Purpose and behavior: Thin wrapper action around `Vampir.HTN.Combat`.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: Runs only if `HTN.Combat.check/1` passes.
- Packet interactions: None directly; operator handles all packets.
- Thresholds, constants, config values: None local.
- How it interacts with other components: Utility action for combat-oriented plans.

#### `Vampir.HTN.Actions.KillOne`
- File path: `lib/vampir/htn/actions/kill_one.ex`
- Purpose and behavior: Primary multi-tick combat action for killing one target.
- Key functions/callbacks: `step/2`, `describe/1`, several private helpers for target refresh, movement, attacking, and result processing.
- Decision logic / priority / conditions: Keeps a sticky target until it is dead/gone/untargetable, approaches first contact via navmesh pathing, attacks when inside min skill range, and finishes when the target disappears, dies, or becomes a zombie. If the target is out of range it pushes `MoveTo` with a stop distance based on skill range.
- Packet interactions: Sends `PktSupportModeStopRequest` once, blocks client support-mode stop traffic, sends `PktSkillStart` attack payloads, and consumes `608` skill result tuples from `GameState`.
- Thresholds, constants, config values: Attack cooldown `750 ms`; approach stop distance `150`; computed attack range `Tables.Skill.min_range() - 50`; maintains `:pending_attacks`.
- How it interacts with other components: Uses `Tables.Skill`, `Pathfinder`, `MoveTo`, `GameState`, and `CombatStats`.

#### `Vampir.HTN.Methods.SoulAbsorb`
- File path: `lib/vampir/htn/methods/soul_absorb.ex`
- Purpose and behavior: High-priority goal for corpse/zombie absorb follow-up.
- Key functions/callbacks: `__goals__/0`, `applicable?/2`, `decompose/3`.
- Decision logic / priority / conditions: Goal `:soul_absorb` priority `72`; applicable when any zombie actor is visible.
- Packet interactions: Decomposes to `MoveTo` then `SoulAbsorb`.
- Thresholds, constants, config values: Uses an `800` unit proximity gate for absorb.
- How it interacts with other components: Depends on `GameState.is_zombie?/1` and `Actions.SoulAbsorb`.

#### `Vampir.HTN.Methods.SoulAbsorbNormal`
- File path: `lib/vampir/htn/methods/soul_absorb_normal.ex`
- Purpose and behavior: Goal for normal mobs that expose effect `23`.
- Key functions/callbacks: `__goals__/0`, `applicable?/2`, `decompose/3`.
- Decision logic / priority / conditions: Goal `:soul_absorb_normal` priority `71`; applicable when an absorbable actor with effect `23` is within range.
- Packet interactions: Decomposes to `MoveTo` then `SoulAbsorbNormal`.
- Thresholds, constants, config values: Uses an `800` unit proximity gate.
- How it interacts with other components: Depends on `GameState.is_soul_absorbable?/1` and `Actions.SoulAbsorbNormal`.

#### `Vampir.HTN.Actions.SoulAbsorb`
- File path: `lib/vampir/htn/actions/soul_absorb.ex`
- Purpose and behavior: Executes the zombie absorb skill.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: Requeues `MoveTo` if out of range; marks actors as absorbed to avoid duplicate work.
- Packet interactions: Sends `PktSkillStart` for skill `400003`.
- Thresholds, constants, config values: Range `850`; cooldown `1500 ms`.
- How it interacts with other components: Used by `Methods.SoulAbsorb`; relies on `MoveTo` and live actor state.

#### `Vampir.HTN.Actions.SoulAbsorbNormal`
- File path: `lib/vampir/htn/actions/soul_absorb_normal.ex`
- Purpose and behavior: Executes the normal absorb skill for effect-23 mobs.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: Same structure as `SoulAbsorb`.
- Packet interactions: Sends `PktSkillStart` for skill `1115`.
- Thresholds, constants, config values: Range `850`; cooldown `1500 ms`.
- How it interacts with other components: Used by `Methods.SoulAbsorbNormal`.

#### `Vampir.HTN.Actions.CircleKite`
- File path: `lib/vampir/htn/actions/circle_kite.ex`
- Purpose and behavior: Experimental/sidecar combat action that orbits around an anchor point while attacking.
- Key functions/callbacks: `step/2`, `describe/1`, private orbit, dash, potion, and result helpers.
- Decision logic / priority / conditions: Builds an orbit anchor on init, prefers aggro targets, can dash toward distant targets, and auto-potions below a health ratio threshold. This action is not currently selected by any main HTN method, but it is implemented and logged.
- Packet interactions: Sends `PktCharacterMove`, `PktSkillStart`, and `PktCombatItemUse`; consumes pending `608` results.
- Thresholds, constants, config values: Anchor radius `1200`; attack range `750`; attack cooldown `400 ms`; dash skill `13001`; dash threshold `1200`; dash cooldown `20_000 ms`; timeout `3_000 ms`; potion ratio `0.5`; potion cooldown `10_000 ms`.
- How it interacts with other components: Uses `GameState`, `Tables.Skill`, and combat event logging.

#### `Vampir.HTN.Actions.BossKite`
- File path: `lib/vampir/htn/actions/boss_kite.ex`
- Purpose and behavior: Experimental/sidecar boss kiting action that circle-strafes a fixed target.
- Key functions/callbacks: `step/2`, `describe/1`, private target acquisition, kite-direction math, attack, and potion helpers.
- Decision logic / priority / conditions: Maintains a target, flips orbit direction periodically, attacks inside range, and auto-potions. Like `CircleKite`, it exists in the codebase but is not currently wired into a planner method.
- Packet interactions: Sends `PktCharacterMove`, `PktSkillStart`, and `PktCombatItemUse`.
- Thresholds, constants, config values: Attack range `350`; kite min/ideal/max `100/200/300`; search range `5000`; attack cooldown `750 ms`; orbit flip `8_000 ms`; timeout `300_000 ms`; potion ratio `0.5`; potion cooldown `10_000 ms`.
- How it interacts with other components: Uses `Tables.Skill`, `GameState`, and pending-attack tracking.

#### `Vampir.Tables.Skill`
- File path: `lib/vampir/tables/skill.ex`
- Purpose and behavior: Loads skill metadata and holds current-class skill state plus client-side cooldown tracking.
- Key functions/callbacks: `init/0`, `init_for_class/1`, `get/1`, `min_range/0`, `available?/2`, `pick_available/1`, `record_use/2`, `record_cooldown_hit/2`, `filter_active_form/2`.
- Decision logic / priority / conditions: Rotates through equipped skills, preferring off-cooldown real skills, then cycling normal attacks. It filters skills based on whether the player is armed or unarmed and ignores non-player/system skill ids.
- Packet interactions: None directly; its state determines which skill id combat actions send in `PktSkillStart`.
- Thresholds, constants, config values: Class skill id ranges are `{110001..110099}`, `{120001..120099}`, `{130001..130099}`; default cooldown fallback for unknown `505` responses is `8_000 ms`.
- How it interacts with other components: Used by `KillOne`, `HTN.Combat`, `BossKite`, `CircleKite`, and `GameState` transform-skill loading.

#### `Vampir.HTN.CombatStats`
- File path: `lib/vampir/htn/combat_stats.ex`
- Purpose and behavior: Lightweight combat event store for debugging and tuning.
- Key functions/callbacks: event loggers and summary functions over multiple time windows.
- Decision logic / priority / conditions: No planning logic.
- Packet interactions: Indirect only; actions log interpreted combat events rather than packets.
- Thresholds, constants, config values: Keeps up to `10_000` events and summarizes windows `5s`, `30s`, `60s`, `5m`, and `1h`.
- How it interacts with other components: Used by `KillOne` and the kite actions.

### 4. Movement

#### `Vampir.HTN.Actions.MoveTo`
- File path: `lib/vampir/htn/actions/move_to.ex`
- Purpose and behavior: Main path-following movement action used by combat, questing, and merchant travel.
- Key functions/callbacks: `step/2`, `describe/1`, plus private steering, heading blending, auto-mount, and finish helpers.
- Decision logic / priority / conditions: Initializes a walk state from current position and path; advances through waypoints when within the waypoint radius; periodically sends move packets based on `gs.move_interval`; stops once final distance is within `stop_dist`. When the path is long enough and the character is not dead or inside an instance, it auto-mounts before travel and dismounts after finishing.
- Packet interactions: Sends `PktCharacterMove`; sends `PktVehicleRiding` for mount toggles; injects S->C `PktWorldMoveFinishResult` via `Inject.teleport_client/3` when the client visual position drifts too far from injected server-side state.
- Thresholds, constants, config values: Waypoint reach `5`; max turn rate `4 rad/s`; default speed `500`; vehicle fallback speed `1000`; mount distance `500`; base move timeout `60_000 ms`; client sync distance `500`.
- How it interacts with other components: Uses `GameState.set_local_pos/1`, `Tables.Vehicle.speed/1`, `Methods.Quest.in_instance?/1`, and `Inject.teleport_client/3`.

#### `Vampir.HTN.Actions.QuestMove`
- File path: `lib/vampir/htn/actions/quest_move.ex`
- Purpose and behavior: Higher-level travel action for quest destinations.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: If inside an instance it only attempts local navmesh/direct movement and never task teleports. Outside instances it first asks `Portals.find_route/2`; if that fails but the destination is within `10_000` units it uses a direct `MoveTo`; otherwise, if a `task_id` exists, it sends a type-21 quest task teleport and waits for `world_changed`.
- Packet interactions: Delegates to `MoveTo`, `LiftTravel`, or sends `PktWorldMoveCast` type `21`.
- Thresholds, constants, config values: Direct-walk fallback distance `10_000`; `world_changed` wait timeout `15_000 ms`.
- How it interacts with other components: Used heavily by `Methods.Quest`, `GadgetInteract`, and `QuestGrind`; depends on `Portals`, `Pathfinder`, `MoveTo`, and `WaitUntil`.

#### `Vampir.HTN.Actions.LiftTravel`
- File path: `lib/vampir/htn/actions/lift_travel.ex`
- Purpose and behavior: Executes the gadget interaction needed to use a lift/portal edge.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: Interacts with the entry gadget, then waits until either the arrival point is near or the player is clearly far from the entry. If the first interaction appears to stall it retries once.
- Packet interactions: Sends `PktGadgetControlStart`.
- Thresholds, constants, config values: Arrival proximity `1000`; “far from entry” threshold `5000`; retry after `5_000 ms`; nil-position timeout `15_000 ms`.
- How it interacts with other components: Produced by `QuestMove` when `Portals.find_route/2` returns lift segments.

#### `Vampir.HTN.Actions.QuestGrind`
- File path: `lib/vampir/htn/actions/quest_grind.ex`
- Purpose and behavior: Patrol-and-fight loop for kill quests.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: Continues until there is no active kill quest. If quest progress stops changing for too long it fails as stuck. If a target is visible it pushes `KillOne`; otherwise it patrols random points around the quest combat area or current position.
- Packet interactions: Indirect only through `KillOne` and `MoveTo`.
- Thresholds, constants, config values: Stuck timeout `120_000 ms`; patrol radius `3000`.
- How it interacts with other components: Uses `HTN.Combat.active_kill_quest/1`, `HTN.Combat.quest_combat_area/1`, `Tables.Quest`, `Pathfinder`, and `MoveTo`.

#### `Vampir.HTN.Actions.Wait`
- File path: `lib/vampir/htn/actions/wait.ex`
- Purpose and behavior: Passive timer action used heavily by movement, teleports, and quest pacing.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None beyond waiting for the deadline.
- Packet interactions: None.
- Thresholds, constants, config values: Uses caller-supplied duration.
- How it interacts with other components: Shared timing primitive.

#### `Vampir.HTN.Actions.WaitUntil`
- File path: `lib/vampir/htn/actions/wait_until.ex`
- Purpose and behavior: Polls movement/world predicates such as `world_changed` and `walk_done`.
- Key functions/callbacks: `step/2`, private `check_condition/3`.
- Decision logic / priority / conditions: Times out if the condition never becomes true.
- Packet interactions: None directly.
- Thresholds, constants, config values: Per-call timeout.
- How it interacts with other components: Used after `MoveTo`, `QuestMove`, `TeleportHome`, and `CharSelect`.

#### `Vampir.HTN.Actions.AwaitPkt`
- File path: `lib/vampir/htn/actions/await_pkt.ex`
- Purpose and behavior: Packet-level wait action for movement/teleport/dungeon acknowledgements.
- Key functions/callbacks: `new/5`, `step/2`.
- Decision logic / priority / conditions: Success is based on packet observation rather than world-state polling.
- Packet interactions: Watches `PacketLog` for expected responses.
- Thresholds, constants, config values: Per-call timeout.
- How it interacts with other components: Created by packet-injecting actions.

#### `Vampir.Pathfinder`
- File path: `lib/vampir/pathfinder.ex`
- Purpose and behavior: Finds local walk paths on the navmesh grid.
- Key functions/callbacks: `find_path/2`, `find_path_elixir/2`.
- Decision logic / priority / conditions: Uses the Rust NIF when available; otherwise falls back to Elixir A*. Before running A* it snaps endpoints to nearby walkable cells and bails out early if the start and goal are in different connected components.
- Packet interactions: None.
- Thresholds, constants, config values: Max A* iterations `50_000`; diagonal cost `1.414`; nearest-walkable snap radius `10`; line-of-sight smoothing samples every half cell.
- How it interacts with other components: Core dependency of `MoveTo`, `QuestMove`, `WalkToMerchant`, `QuestGrind`, `LiftTravel`, and `Portals`.

#### `Vampir.NavMesh`
- File path: `lib/vampir/nav_mesh.ex`
- Purpose and behavior: Loads the walkability grid and polygon data, exposes walkability/component queries, and can mark observed walkable cells from packets.
- Key functions/callbacks: `walkable?/1`, `world_to_grid/1`, `grid_to_world/1`, `same_component?/2`, `contour_repulsion/2`, `cells_in_viewport/3`, `reload/0`, `build_from_captures/0`.
- Decision logic / priority / conditions: Not a planner, but movement logic depends on its connected-component check and walkable lookup. If no component data exists it conservatively returns `true` for `same_component?/2` so pathfinding can still try.
- Packet interactions: Indirect only; subscribes to packet events and can mark walkable observations.
- Thresholds, constants, config values: Default cell size `100.0`; sector size `5000`; Nocturne bounds `x 92000..132000`, `y 67000..132000`.
- How it interacts with other components: Used by `Pathfinder`; indirectly affects every movement-producing HTN action.

#### `Vampir.Portals`
- File path: `lib/vampir/portals.ex`
- Purpose and behavior: Loads lift spawn points and lift graph edges, then finds multi-segment routes that combine walking and lifts.
- Key functions/callbacks: `find_route/2`, `spawns/0`, `edges/0`.
- Decision logic / priority / conditions: Tries direct local pathfinding first; otherwise runs a BFS over lift entry/exit pairs and splices walk segments around them.
- Packet interactions: None directly; resulting `:lift` segments are consumed by `LiftTravel`.
- Thresholds, constants, config values: BFS max depth `5`; candidate lift proximity threshold `50_000.0`.
- How it interacts with other components: Used by `QuestMove`.

#### `Vampir.Tables.NpcSpawn`
- File path: `lib/vampir/tables/npc_spawn.ex`
- Purpose and behavior: Loads exact NPC spawn positions and the spawn-id to npc-info-id map.
- Key functions/callbacks: `spawns/1`, `nearest/2`, `npc_info_id/1`, `spawn_pos/1`.
- Decision logic / priority / conditions: None; lookup only.
- Packet interactions: None.
- Thresholds, constants, config values: Reads `NpcSpawn.csv`.
- How it interacts with other components: Used by `BuyPotions`, `WalkToMerchant`, `TalkTo`, and various quest movement helpers.

### 5. Maintenance, Shop, Equipment, and Mount Handling

#### `Vampir.HTN.Methods.Resupply`
- File path: `lib/vampir/htn/methods/resupply.ex`
- Purpose and behavior: Goal for buying potions when stock is low.
- Key functions/callbacks: `__goals__/0`, `applicable?/2`, `decompose/3`.
- Decision logic / priority / conditions: Goal `:resupply` priority `60`; applicable when `Sensors.low_potions` is true. If already near a merchant it buys immediately; otherwise it teleports home and walks to the merchant first.
- Packet interactions: Decomposes to `TeleportHome`, `WalkToMerchant`, and `BuyPotions`.
- Thresholds, constants, config values: Sleeps `10_000 ms` after teleport and `3_000 ms` after walking/buying.
- How it interacts with other components: Uses `Sensors` and the related merchant/potion operators.

#### `Vampir.HTN.Methods.Maintenance`
- File path: `lib/vampir/htn/methods/maintenance.ex`
- Purpose and behavior: Collects all non-quest maintenance goals.
- Key functions/callbacks: `__goals__/0`, `applicable?/2`, `decompose/3`.
- Decision logic / priority / conditions: Priorities are `combine_cards 300`, `redraw_card 299`, `redraw_mount 298`, `auto_equip 73`, `register_collections 42`, `decompose_junk 41`, `refresh_inventory 30`, `claim_achievements_periodic 29`, `use_summon_tickets 28`, `enhance_equipment 26`, `refine_equipment 25`, `buy_skill_books 23`, `use_skill_books 22`, `maintenance 20`. It stamps `:last_inventory_refresh` and `:last_achievement_claim` when those plans are chosen.
- Packet interactions: Indirect through actions/operators only.
- Thresholds, constants, config values: Most decompositions add `500-3000 ms` sleeps after the operator action.
- How it interacts with other components: Relies heavily on `Sensors` and the maintenance actions/operators below.

#### `Vampir.HTN.Teleport`
- File path: `lib/vampir/htn/operators/teleport.ex`
- Purpose and behavior: Encodes `PktWorldMoveCast` teleports for home, zone travel, or quest-task teleport.
- Key functions/callbacks: `run_home/1`, `run_zone/2`, `run_quest_task/2`.
- Decision logic / priority / conditions: No local checks; the caller decides when teleport is valid.
- Packet interactions: Sends `PktWorldMoveCast` with type `2` (home waypoint), `13` (zone travel), or `21` (quest task teleport).
- Thresholds, constants, config values: Home town id `10_011_001`.
- How it interacts with other components: Used by `TeleportHome`, `QuestMove`, `WalkToMerchant`, and various methods.

#### `Vampir.HTN.Actions.TeleportHome`
- File path: `lib/vampir/htn/actions/teleport_home.ex`
- Purpose and behavior: Thin wrapper around `Vampir.HTN.Teleport.run_home/1`.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None local.
- Packet interactions: None directly; operator sends `PktWorldMoveCast`.
- Thresholds, constants, config values: None local.
- How it interacts with other components: Used by `Methods.Resupply` and `Methods.Quest` shop flow.

#### `Vampir.HTN.BuyPotions`
- File path: `lib/vampir/htn/operators/buy_potions.ex`
- Purpose and behavior: Buys a large potion batch from a general merchant.
- Key functions/callbacks: `check_low_potions/1`, `check_near_shop/1`, `run/1`.
- Decision logic / priority / conditions: Buys only when potion count is below the minimum and a cooldown has elapsed. It computes the purchase quantity as `target - current_count`.
- Packet interactions: Sends `PktShopItemPurchase` for shop type `5` and product `10_500_003`.
- Thresholds, constants, config values: Target potions `500`; min potions `50`; merchant range `500.0`; cooldown `3_000 ms`; merchant NPC ids `[203, 10130, 10231, 91031, 91033, 10610, 10616, 10617, 10618]`; local level check uses stat `158`, which differs from `Sensors` using stat `3`.
- How it interacts with other components: Wrapped by `Actions.BuyPotions`; used by `Methods.Resupply` and the quest shop flow.

#### `Vampir.HTN.Actions.BuyPotions`
- File path: `lib/vampir/htn/actions/buy_potions.ex`
- Purpose and behavior: Thin wrapper around `Vampir.HTN.BuyPotions`.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None local.
- Packet interactions: None directly; operator sends `PktShopItemPurchase`.
- Thresholds, constants, config values: None local.
- How it interacts with other components: Used by `Methods.Resupply`.

#### `Vampir.HTN.WalkToMerchant`
- File path: `lib/vampir/htn/operators/walk_to_merchant.ex`
- Purpose and behavior: Gets the character to the town merchant, teleporting home first if necessary.
- Key functions/callbacks: `check/1`, `run/1`.
- Decision logic / priority / conditions: Only runs when idle, level is high enough, potion count is low, and the character is not already near a merchant. If `world_info_id != 10011`, it teleports home; otherwise it pathfinds to the nearest merchant spawn in the town world.
- Packet interactions: Indirect only through `Teleport.run_home/1` and `MoveTo`.
- Thresholds, constants, config values: Town world `10011`; merchant range `500.0`; min potions `50`; level stat `158`; min buy level `10`.
- How it interacts with other components: Wrapped by `Actions.WalkToMerchant`; depends on `NpcSpawn` and `Pathfinder`.

#### `Vampir.HTN.Actions.WalkToMerchant`
- File path: `lib/vampir/htn/actions/walk_to_merchant.ex`
- Purpose and behavior: Thin wrapper around `Vampir.HTN.WalkToMerchant`.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None local.
- Packet interactions: None directly.
- Thresholds, constants, config values: None local.
- How it interacts with other components: Used by `Methods.Resupply` and `Methods.Quest`.

#### `Vampir.HTN.BuySkillBooks`
- File path: `lib/vampir/htn/operators/buy_skill_books.ex`
- Purpose and behavior: Opens the skill-book shop and buys the next unbought class/tier book.
- Key functions/callbacks: `check/1`, `run/1`, `has_buyable?/0`.
- Decision logic / priority / conditions: Uses `Tables.Shop.next_book/3` with class id, level, and the process-local `:bought_skill_book_tiers` set. It backs off after failures and records purchased tiers.
- Packet interactions: Sends `PktShopItemListRead` to open the listing, then `PktShopItemPurchase`; inspects recent `PktShopItemPurchaseResult` packets in `PacketLog` to confirm the result code.
- Thresholds, constants, config values: Shop type `6`; shop tab `6`; failure cooldown `60_000 ms`; declared buy cooldown `5_000 ms` exists but is not used in the current operator logic.
- How it interacts with other components: Wrapped by `Actions.BuySkillBooks`; uses `Tables.Shop`, `PacketLog`, and `GameState`.

#### `Vampir.HTN.Actions.BuySkillBooks`
- File path: `lib/vampir/htn/actions/buy_skill_books.ex`
- Purpose and behavior: Thin wrapper around `Vampir.HTN.BuySkillBooks`.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None local.
- Packet interactions: None directly.
- Thresholds, constants, config values: None local.
- How it interacts with other components: Used by `Methods.Maintenance`.

#### `Vampir.HTN.UseSkillBook`
- File path: `lib/vampir/htn/operators/use_skill_book.ex`
- Purpose and behavior: Uses the first inventory item whose icon looks like a skill book.
- Key functions/callbacks: `check/1`, `run/1`, `has_skill_books?/0`.
- Decision logic / priority / conditions: Searches all inventory tabs and backs off after recent failures.
- Packet interactions: Sends `PktItemUse` payload `<<template_id::le32, item_id::le64, 1::le16, 0::le32>>`.
- Thresholds, constants, config values: Failure cooldown `30_000 ms`; skill books are detected by icon containing `SkillBook`.
- How it interacts with other components: Wrapped by `Actions.UseSkillBook`; surfaced through `Sensors.has_skill_books`.

#### `Vampir.HTN.Actions.UseSkillBook`
- File path: `lib/vampir/htn/actions/use_skill_book.ex`
- Purpose and behavior: Thin wrapper around `Vampir.HTN.UseSkillBook`.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None local.
- Packet interactions: None directly.
- Thresholds, constants, config values: None local.
- How it interacts with other components: Used by `Methods.Maintenance`.

#### `Vampir.HTN.UseSummonTicket`
- File path: `lib/vampir/htn/operators/use_summon_ticket.ex`
- Purpose and behavior: Uses the first mount/costume/sephira draw ticket found in inventory.
- Key functions/callbacks: `check/1`, `run/1`, `has_tickets?/0`.
- Decision logic / priority / conditions: Ticket detection is icon-based and searches all inventory tabs.
- Packet interactions: Sends `PktItemUse` with the same payload shape as `UseSkillBook`.
- Thresholds, constants, config values: Cooldown `3_000 ms`; ticket icons include `VehicleDraw`, `CostumeDraw`, or `SephiraDraw`.
- How it interacts with other components: Wrapped by `Actions.UseSummonTicket`; used by `Methods.Maintenance`.

#### `Vampir.HTN.Actions.UseSummonTicket`
- File path: `lib/vampir/htn/actions/use_summon_ticket.ex`
- Purpose and behavior: Thin wrapper around `Vampir.HTN.UseSummonTicket`.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None local.
- Packet interactions: None directly.
- Thresholds, constants, config values: None local.
- How it interacts with other components: Used by `Methods.Maintenance`.

#### `Vampir.HTN.Actions.AutoEquip`
- File path: `lib/vampir/htn/actions/auto_equip.ex`
- Purpose and behavior: Automatically equips better costumes, vehicles, and gear.
- Key functions/callbacks: `check/1`, `step/2`, `on_equip_result/2`.
- Decision logic / priority / conditions: Prioritizes costume upgrades first, then vehicle upgrades, then gear. Gear comparison is `rarity > tier > enhance`, subject to slot validity and class compatibility. It tracks attempted items and blacklists those that come back with failed equip results.
- Packet interactions: Sends `PktCollectionEquip` for costume slot `0` / type `0` and vehicle slot `25` / type `2`; sends `PktItemEquip` for gear.
- Thresholds, constants, config values: Cooldown `3_000 ms`; valid gear slots `[5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18]`.
- How it interacts with other components: Triggered via `Methods.Maintenance`; receives `PktItemEquipResult` feedback through `GameState` calling `on_equip_result/2`; uses `ItemData`, `Tables.Costume`, `Tables.Vehicle`, and `GameState.collection_equip`.

#### `Vampir.HTN.CollectionRegister`
- File path: `lib/vampir/htn/operators/collection_register.ex`
- Purpose and behavior: Registers inventory items into collection entries.
- Key functions/callbacks: `check/1`, `run/1`, `seed_registered/1`.
- Decision logic / priority / conditions: Only registers non-equipped items that satisfy the collection template/count/enchant requirement and have rarity below `2`. It skips collection ids already known in `:registered_collections`.
- Packet interactions: Sends `PktCollectionRegister`.
- Thresholds, constants, config values: Cooldown `5_000 ms`; sleeps `200 ms` between registrations; reads `Collection.csv`.
- How it interacts with other components: Wrapped by `Actions.CollectionRegister`; `GameState` seeds initial collection ids through `seed_registered/1`.

#### `Vampir.HTN.Actions.CollectionRegister`
- File path: `lib/vampir/htn/actions/collection_register.ex`
- Purpose and behavior: Thin wrapper around `Vampir.HTN.CollectionRegister`.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None local.
- Packet interactions: None directly.
- Thresholds, constants, config values: None local.
- How it interacts with other components: Used by `Methods.Maintenance`.

#### `Vampir.HTN.CombineCards`
- File path: `lib/vampir/htn/operators/combine_cards.ex`
- Purpose and behavior: Groups cards by combination group and sends one combine batch.
- Key functions/callbacks: `check/1`, `run_batch/1`.
- Decision logic / priority / conditions: Chooses the first target group with at least four cards, preferring higher groups first. Builds a batch from up to four stacks, capping total materials at `40` and forcing a multiple of `4`.
- Packet interactions: Sends `PktItemCombination`.
- Thresholds, constants, config values: Max materials per combine `40`; target groups `[50005001, 50004001, 50003001, 50002001, 50001001, 30005000, 30004000, 30003000, 30002000, 30001000]`.
- How it interacts with other components: Wrapped by `Actions.CombineCards` and `Actions.CombineCardsBatch`; uses `Tables.Combination`.

#### `Vampir.HTN.Actions.CombineCards`
- File path: `lib/vampir/htn/actions/combine_cards.ex`
- Purpose and behavior: Thin wrapper around `Vampir.HTN.CombineCards`.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None local.
- Packet interactions: None directly.
- Thresholds, constants, config values: None local.
- How it interacts with other components: Used by `Methods.Maintenance`.

#### `Vampir.HTN.Actions.CombineCardsBatch`
- File path: `lib/vampir/htn/actions/combine_cards_batch.ex`
- Purpose and behavior: One-shot combine-batch action.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: Updates `:last_combine_cards` when `run_batch/1` combines anything.
- Packet interactions: None directly; operator sends `PktItemCombination`.
- Thresholds, constants, config values: None local.
- How it interacts with other components: Used by `Methods.Maintenance` high-priority `:combine_cards`.

#### `Vampir.HTN.DecomposeJunk`
- File path: `lib/vampir/htn/operators/decompose_junk.ex`
- Purpose and behavior: Disassembles common-rarity gear that is not equipped and not needed for collections.
- Key functions/callbacks: `check/1`, `run/1`.
- Decision logic / priority / conditions: Filters inventory tab `0` for items with `rarity == 0`, `count == 1`, `template_id > 10_000_000`, a non-nil equip type, not equipped, and not present in `Collection.csv`.
- Packet interactions: Sends `PktItemDisassembly`.
- Thresholds, constants, config values: Cooldown `10_000 ms`; batch size `10`.
- How it interacts with other components: Wrapped by `Actions.DecomposeJunk`; uses `ItemData` and `Tables.Item`-style collection membership data.

#### `Vampir.HTN.Actions.DecomposeJunk`
- File path: `lib/vampir/htn/actions/decompose_junk.ex`
- Purpose and behavior: Thin wrapper around `Vampir.HTN.DecomposeJunk`.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None local.
- Packet interactions: None directly.
- Thresholds, constants, config values: None local.
- How it interacts with other components: Used by `Methods.Maintenance`.

#### `Vampir.HTN.EnhanceEquipment`
- File path: `lib/vampir/htn/operators/enhance_equipment.ex`
- Purpose and behavior: Safe-enchants equipped items up to their safe cap.
- Key functions/callbacks: `check/1`, `run/1`.
- Decision logic / priority / conditions: Finds the first equipped item on the active deck that is below its target safe level. Weapons/armor require rarity `>= 2`; accessories require rarity `>= 1`. Green items are capped at `+1`, while blue+ items use the safe cap from `ItemData.safe_enhance_cap/1`.
- Packet interactions: Sends `PktItemEnchant`.
- Thresholds, constants, config values: Cooldown `2_000 ms`; weapon scroll `60000002`; armor scroll `60000003`; accessory scroll `60000004`.
- How it interacts with other components: Wrapped by `Actions.EnhanceEquip`; depends on `ItemData` safe-enchant metadata and `GameState` inventory/equipment updates.

#### `Vampir.HTN.Actions.EnhanceEquip`
- File path: `lib/vampir/htn/actions/enhance_equip.ex`
- Purpose and behavior: Thin wrapper around `Vampir.HTN.EnhanceEquipment`.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None local.
- Packet interactions: None directly.
- Thresholds, constants, config values: None local.
- How it interacts with other components: Used by `Methods.Maintenance`.

#### `Vampir.HTN.RefineEquipment`
- File path: `lib/vampir/htn/operators/refine_equipment.ex`
- Purpose and behavior: Refines equipped blue items that do not already have refine options.
- Key functions/callbacks: `check/1`, `run/1`.
- Decision logic / priority / conditions: Picks the first active-deck equipped item with rarity `2` in the valid weapon/armor/accessory slots and with no `refine_options` present in inventory. It requires enough Trinity material for the slot category.
- Packet interactions: Sends `PktEquipmentRefine`.
- Thresholds, constants, config values: Cooldown `2_000 ms`; material template `100_000_000`; material costs `{weapon, blue}=10`, `{armor, blue}=3`, `{accessory, blue}=15`.
- How it interacts with other components: Wrapped by `Actions.RefineEquip`; depends on inventory item detail stored by `GameState`.

#### `Vampir.HTN.Actions.RefineEquip`
- File path: `lib/vampir/htn/actions/refine_equip.ex`
- Purpose and behavior: Thin wrapper around `Vampir.HTN.RefineEquipment`.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None local.
- Packet interactions: None directly.
- Thresholds, constants, config values: None local.
- How it interacts with other components: Used by `Methods.Maintenance`.

#### `Vampir.HTN.OpenChests`
- File path: `lib/vampir/htn/operators/open_chests.ex`
- Purpose and behavior: Opens chest items when inventory still has room.
- Key functions/callbacks: `check/1`, `run/1`.
- Decision logic / priority / conditions: Picks the chest stack with the largest count. Chest detection is name/icon based and excludes “selection” chests.
- Packet interactions: Sends `PktItemBoxUse`.
- Thresholds, constants, config values: Inventory tab `0`; slot cap `90`; cooldown `1_500 ms`.
- How it interacts with other components: Wrapped by `Actions.OpenChests`; used by `Methods.Maintenance`.

#### `Vampir.HTN.Actions.OpenChests`
- File path: `lib/vampir/htn/actions/open_chests.ex`
- Purpose and behavior: Thin wrapper around `Vampir.HTN.OpenChests`.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None local.
- Packet interactions: None directly.
- Thresholds, constants, config values: None local.
- How it interacts with other components: Used by `Methods.Maintenance`.

#### `Vampir.HTN.Actions.RedrawCard`
- File path: `lib/vampir/htn/actions/redraw_card.ex`
- Purpose and behavior: Uses the card redraw packet on the first redrawable character card.
- Key functions/callbacks: `check/1`, `step/2`, `describe/1`.
- Decision logic / priority / conditions: Looks for items whose combination group is in `30004000` or `30005000`; only runs when the redraw timer has elapsed.
- Packet interactions: Sends `PktItemRedraw`; awaits `859`.
- Thresholds, constants, config values: Redraw cadence `1_800_000 ms` (`30 minutes`).
- How it interacts with other components: Triggered via `Methods.Maintenance`; uses `Tables.Combination` and `ItemData`.

#### `Vampir.HTN.Actions.RedrawMount`
- File path: `lib/vampir/htn/actions/redraw_mount.ex`
- Purpose and behavior: Uses the redraw packet on the first redrawable mount item.
- Key functions/callbacks: `check/1`, `step/2`, `describe/1`.
- Decision logic / priority / conditions: Uses combination groups `50004001` and `50005001`.
- Packet interactions: Sends `PktItemRedraw`; awaits `859`.
- Thresholds, constants, config values: Redraw cadence `1_800_000 ms`.
- How it interacts with other components: Triggered via `Methods.Maintenance`; uses `Tables.Combination`.

#### `Vampir.HTN.Actions.RefreshInventory`
- File path: `lib/vampir/htn/actions/refresh_inventory.ex`
- Purpose and behavior: Requests a fresh inventory list from the server.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None.
- Packet interactions: Sends `PktInventoryListRead` payload `<<0, 0::le32>>`.
- Thresholds, constants, config values: None local; `Methods.Maintenance` triggers it every `60_000 ms`.
- How it interacts with other components: Keeps `GameState.inventory` fresh for maintenance logic.

#### `Vampir.HTN.Actions.ClaimAchievements`
- File path: `lib/vampir/htn/actions/claim_achievements.ex`
- Purpose and behavior: Claims all completable achievements.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None.
- Packet interactions: Sends `PktAchievementRewardAllGet`.
- Thresholds, constants, config values: None local; `Methods.Maintenance` and `Sensors` gate the periodic cadence.
- How it interacts with other components: `GameState` clears `completable_achievements` on successful result packets.

#### `Vampir.Tables.Shop`
- File path: `lib/vampir/tables/shop.ex`
- Purpose and behavior: Parses shop data for class skill books.
- Key functions/callbacks: `skill_books/0`, `buyable_books/2`, `next_book/3`.
- Decision logic / priority / conditions: Filters buyable entries by class and required level, then returns the first tier not present in the bought-tier set.
- Packet interactions: None.
- Thresholds, constants, config values: Skill-book item range `10600001..10600311`; requirement type `0` means level-gated.
- How it interacts with other components: Used by `BuySkillBooks`.

#### `Vampir.Tables.Vehicle`
- File path: `lib/vampir/tables/vehicle.ex`
- Purpose and behavior: Loads vehicle rarity, speed, and name.
- Key functions/callbacks: `all/0`, `get/1`, `speed/1`, `name/1`.
- Decision logic / priority / conditions: None.
- Packet interactions: None.
- Thresholds, constants, config values: Reads `Vehicle.csv` and `L10N/en/Vehicle_Name.csv`.
- How it interacts with other components: Used by `MoveTo` for mounted speed and by `AutoEquip` for mount upgrades.

#### `Vampir.Tables.Costume`
- File path: `lib/vampir/tables/costume.ex`
- Purpose and behavior: Loads costume rarity and backing item ids.
- Key functions/callbacks: `all/0`, `get/1`, `costume?/1`.
- Decision logic / priority / conditions: None.
- Packet interactions: None.
- Thresholds, constants, config values: Reads `Costume.csv`.
- How it interacts with other components: Used by `AutoEquip`, `GameState`, and `Methods.GoldFarm`.

#### `Vampir.Tables.Combination`
- File path: `lib/vampir/tables/combination.ex`
- Purpose and behavior: Maps item template ids to combination ids.
- Key functions/callbacks: `group/1`, `all/0`.
- Decision logic / priority / conditions: None.
- Packet interactions: None.
- Thresholds, constants, config values: Reads `ItemCombinationMaterial.csv`.
- How it interacts with other components: Used by `CombineCards`, `RedrawCard`, and `RedrawMount`.

#### `Vampir.Tables.Item`
- File path: `lib/vampir/tables/item.ex`
- Purpose and behavior: Tracks auction tabs, tradability, and collection membership.
- Key functions/callbacks: `auction_tab/1`, `tradable?/1`, `collection_item?/1`, `collection_tab/1`, `collection_templates/0`.
- Decision logic / priority / conditions: None.
- Packet interactions: None.
- Thresholds, constants, config values: Binds `0` and tradable `2`; reads `Item.csv`, `Collection.csv`, and `CollectionGroup.csv`.
- How it interacts with other components: Relevant to maintenance filtering and inventory semantics.

#### `Vampir.ItemData`
- File path: `lib/vampir/item_data.ex`
- Purpose and behavior: Central item metadata store used by almost every maintenance decision.
- Key functions/callbacks: `get/1`, `name/1`, `icon/1`, `rarity/1`, `equip_type/1`, `tier/1`, `class_info_id/1`, `safe_enhance_cap/1`.
- Decision logic / priority / conditions: No planner logic, but `AutoEquip`, `EnhanceEquipment`, `RefineEquipment`, `UseSkillBook`, `UseSummonTicket`, and `OpenChests` all rely on its rarity/type/icon/class/tier data.
- Packet interactions: None.
- Thresholds, constants, config values: Loads safe enchant caps from `ItemEnchant.csv` as `(max safe level) + 1`, keyed by `{equip_type, rarity}`.
- How it interacts with other components: Foundational metadata layer for maintenance and gear comparison.

### 6. Instance / Dungeon Handling and Special Automation

#### `Vampir.HTN.Methods.Survive`
- File path: `lib/vampir/htn/methods/survive.ex`
- Purpose and behavior: Death-recovery method.
- Key functions/callbacks: `__goals__/0`, `applicable?/2`, `decompose/3`.
- Decision logic / priority / conditions: Goal `:revive` priority `90`; if dead inside an instance it exits the world instead of reviving in place.
- Packet interactions: Decomposes to `WorldExit` or `Revive`.
- Thresholds, constants, config values: Sleeps `5_000 ms` after `WorldExit` and `1_000 ms` after `Revive`.
- How it interacts with other components: Depends on `Methods.Quest.in_instance?/1`, `Actions.WorldExit`, and `Actions.Revive`.

#### `Vampir.HTN.Revive`
- File path: `lib/vampir/htn/operators/revive.ex`
- Purpose and behavior: Operator-style revive loop with resend cooldown and hard timeout.
- Key functions/callbacks: `check/1`, `run/1`.
- Decision logic / priority / conditions: Clears revive state if the player is no longer dead. While dead, it resends revive only after cooldown and gives up after a time wall.
- Packet interactions: Sends `PktCharacterRevive` payload `<<1>>`.
- Thresholds, constants, config values: Cooldown `5_000 ms`; time wall `30_000 ms`.
- How it interacts with other components: Wrapped by `Actions.Revive`; `GameState` clears `dead` on revive packets.

#### `Vampir.HTN.Actions.Revive`
- File path: `lib/vampir/htn/actions/revive.ex`
- Purpose and behavior: One-shot revive action with response wait.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: None.
- Packet interactions: Sends `PktCharacterRevive`; awaits `319`.
- Thresholds, constants, config values: Timeout `10_000 ms`.
- How it interacts with other components: Used by `Methods.Survive`.

#### `Vampir.HTN.Methods.WeepingVillage`
- File path: `lib/vampir/htn/methods/weeping_village.ex`
- Purpose and behavior: Hardcoded controller for the Weeping Village party dungeon.
- Key functions/callbacks: `__goals__/0`, `applicable?/2`, `decompose/3`.
- Decision logic / priority / conditions: Goal `:dungeon_weeping_village` priority `72`; only applies in world `50010` while quest `5000001001` is active. On kill steps it fights nearby hostiles; on non-kill steps it throttles dungeon quest updates and moves to hardcoded positions keyed by descending step numbers.
- Packet interactions: Decomposes to `KillOne`, `MoveTo`, `DungeonQuestUpdate`, `Sleep`, and `ExitDungeon`.
- Thresholds, constants, config values: NPC scan range `5000.0`; quest-update throttle `3_000 ms`; uses a hardcoded `@waypoints` map by step.
- How it interacts with other components: Overrides generic quest behavior for this dungeon.

#### `Vampir.HTN.Methods.GoldFarm`
- File path: `lib/vampir/htn/methods/gold_farm.ex`
- Purpose and behavior: High-priority special automation mode gated by the presence of `.enable_farm`.
- Key functions/callbacks: `__goals__/0`, `applicable?/2`, `decompose/3`.
- Decision logic / priority / conditions: When enabled, exposes priorities `200` for `:gold_farm`, `:tutorial_character_card_draw`, and `:tutorial_mount_card_draw`. `:gold_farm` is currently hard-disabled by `applicable?(:gold_farm, _) -> false`; the two tutorial goals fire only if the corresponding tutorial quests are active and the player lacks a purple costume/vehicle.
- Packet interactions: Delegates to `GoldFarmTick`, `TutorialCharacterCardDraw`, and `TutorialMountCardDraw`.
- Thresholds, constants, config values: Gold quest id `110720`; character card quest `110530`; mount quest `110650`; gold stat `21`; gold target `1_000_000`.
- How it interacts with other components: Uses `Tables.Costume`, vehicle collection state from `GameState`, and the tutorial actions.

#### `Vampir.HTN.Actions.GoldFarmTick`
- File path: `lib/vampir/htn/actions/gold_farm_tick.ex`
- Purpose and behavior: Multi-tick farm loop that spams tutorial rewards and disassembles rusty body armor.
- Key functions/callbacks: `step/2`, private reward/sell helpers.
- Decision logic / priority / conditions: Stops once gold stat `21` reaches `1_000_000` or once the yield interval elapses so the planner can reevaluate.
- Packet interactions: Sends batched `PktTutorialUpdate` packets for info id `4007` steps `2` and `3`; sends `PktItemDisassembly` for template `12021000`.
- Thresholds, constants, config values: Reward interval `500 ms`; sell/disassembly interval `1_000 ms`; yield interval `4_000 ms`.
- How it interacts with other components: Used only by `Methods.GoldFarm`.

#### `Vampir.HTN.Actions.TutorialCharacterCardDraw`
- File path: `lib/vampir/htn/actions/tutorial_character_card_draw.ex`
- Purpose and behavior: Multi-tick action that repeatedly advances tutorial reward steps for the costume draw tutorial.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: Runs only while quest `110530` is active and yields after a fixed interval.
- Packet interactions: Sends batched `PktTutorialUpdate` packets for info id `4009` steps `2` and `3`.
- Thresholds, constants, config values: Send interval `250 ms`; yield interval `4_000 ms`.
- How it interacts with other components: Used by `Methods.GoldFarm`.

#### `Vampir.HTN.Actions.TutorialMountCardDraw`
- File path: `lib/vampir/htn/actions/tutorial_mount_card_draw.ex`
- Purpose and behavior: Multi-tick action that repeatedly advances tutorial reward steps for the mount draw tutorial.
- Key functions/callbacks: `step/2`, `describe/1`.
- Decision logic / priority / conditions: Runs only while quest `110650` is active and yields after a fixed interval.
- Packet interactions: Sends batched `PktTutorialUpdate` packets for info id `4008` steps `2` and `3`.
- Thresholds, constants, config values: Send interval `250 ms`; yield interval `4_000 ms`.
- How it interacts with other components: Used by `Methods.GoldFarm`.

### 7. State Tracking, Packet Handling, and Recovery

#### `Vampir.GameState`
- File path: `lib/vampir/game_state.ex`
- Purpose and behavior: Central packet-driven state store for the HTN system. It maintains player/world position, actors, gadgets, quests, stats, aggro, inventory, equipment, skills, mounts, collections, achievements, and combat result queues.
- Key functions/callbacks: `initial_state/0`, `publish/2`, `get_state/0`, `process_packet/2`, `set_local_pos/1`, `tick_actors/1`, `drain_skill_results/1`, `clear/0`.
- Decision logic / priority / conditions: It intentionally treats `1811` and `1813` quest packets as more authoritative than bulk quest sync and avoids letting bulk sync regress `current_step`, `progress`, or `progress_target`. It clears actors/gadgets on world transitions and revive notifications. It also ignores normal skill-list packets when a transform-skill set is active.
- Packet interactions: 
  - World/position: `PktCharacterMove`, `PktCharacterMoveNotify`, `PktCharacterMoveListNotify`, `PktWorldMoveFinishResult`, `PktLiftNotify`, `PktNpcMoveNotify`, `PktNpcTeleportNotify`.
  - Quest: `PktQuestSyncNotify`, `PktQuestUpdateNotify`, `PktQuestStartResult`, `PktQuestUpdateResult`, `PktQuestProgressUpdateNotify`, `PktQuestLimitTimeNotify`, opcode `1847` quest removal, `PktQuestFailNotify`, `PktQuestCompleteResult`, `PktTutorialStartResult`, `PktQuestTeleportResult`.
  - Combat/skills: `PktSkillStartNotify`, `PktSkillStartResult`, `PktSkillHitNotify`, effect opcodes `613` and `614`, `PktNpcTargetUpdateNotify`.
  - Inventory/equipment/maintenance: `PktInventoryListReadResult`, `PktEquipListReadResult`, `PktItemEquipResult`, `PktItemUnequipResult`, `PktItemEnchantResult`, `PktItemUseResult`, `PktCombatItemUseResult`, `PktShopItemPurchaseResult`, `PktItemCombinationResult`, `PktItemRedrawResult`, `PktItemDisassemblyResult`, `PktItemBoxUseResult`, `PktAssetItemChangeNotify`, `PktItemLootNotify`, achievement packets, collection packets, and vehicle riding packets.
- Thresholds, constants, config values: ETS ring size `100_000`; skill result queue size `20`; effect ids `6501971` and `65019411` for zombie, `23` for absorbable, `6501951` for untargetable; NPC interpolation speed `800`; default move interval `200`.
- How it interacts with other components: `Sensors`, combat actions, movement actions, `Methods.Quest`, and all maintenance logic read from it. `Inject` depends on cached `206` and `355` templates stored by `GameState`.

#### `Vampir.NpcData`
- File path: `lib/vampir/npc_data.ex`
- Purpose and behavior: Loads NPC display metadata and caches runtime `NpcId -> NpcInfoId` mappings.
- Key functions/callbacks: `get/1`, `display_name/1`, `npc_type/1`, `level/1`, `max_hp/1`, `cache_npc_id/2`, `lookup_npc_id/1`.
- Decision logic / priority / conditions: None.
- Packet interactions: None directly, but `GameState` populates runtime cache from spawn/create packets.
- Thresholds, constants, config values: Reads `Npc.csv`, `NpcBattle.csv`, and `L10N/en/Npc_Name.csv`.
- How it interacts with other components: Used by `GameState` to annotate visible actors and by quest/combat logic that filters NPCs by type/name.

#### Cross-cutting packet trigger map
- File path: Multiple modules, mainly `lib/vampir/game_state.ex`, `lib/vampir/proxy/handler.ex`, `lib/vampir/htn/actions/*.ex`, and `lib/vampir/htn/operators/*.ex`
- Purpose and behavior: This is the effective packet-to-HTN event contract that a Rust port needs to preserve.
- Key functions/callbacks:
  - `206 PktWorldMoveFinishResult`: authoritative world entry/teleport completion; resets actors/gadgets, sets `world_id`, `world_info_id`, `zone_quest_id`, position, and transform skills; unblocks quest/movement plans waiting on world change.
  - `1811 PktQuestUpdateResult` and `1813 PktQuestProgressUpdateNotify`: authoritative story step/progress updates; these drive `next_waypoint/1`, quest completion, and many interact/wait retries.
  - `1840 PktQuestLimitTimeNotify`: writes `current_task_id`, which matters for hybrid quests where `current_step` alone is not enough.
  - `1847` quest removal: marks `dungeon_expired = true`, which triggers goal `:exit_expired_dungeon`.
  - `326 PktNpcTargetUpdateNotify`: updates `aggro_npcs`, which immediately feeds combat goal selection and target priority.
  - `608 PktSkillStartResult`: pushes `{result, tid}` into the skill result queue; combat code uses this to confirm attacks, learn cooldown hits (`505`), and blacklist invalid targets.
  - `613/614` effect add/remove: mark mobs as zombie, absorbable, or untargetable; this directly affects soul-absorb goals and combat target filtering.
  - `8002/8003` vehicle riding packets: update mounted state and vehicle id, which changes movement speed and mount behavior.
  - Achievement and collection result packets: trigger periodic maintenance goals and update auto-equip inputs.
- Decision logic / priority / conditions: The handler publishes packets into `GameState` before filtering, so even blocked client packets can still update internal state; this is especially important for dropped client `305` movement packets while HTN controls movement.
- Packet interactions: See the bullet map above.
- Thresholds, constants, config values: HTN tick cadence is driven by `gs.move_interval` when available.
- How it interacts with other components: This trigger map is what ties packets to sensors, goals, retries, and action completion.

#### Cross-cutting error handling and recovery rules
- File path: Multiple modules, mainly `lib/vampir/htn/action_executor.ex`, `lib/vampir/htn/actions/*.ex`, `lib/vampir/htn/operators/*.ex`, and `lib/vampir/proxy/handler.ex`
- Purpose and behavior: Collects the non-obvious retry, timeout, and backoff rules that make the current HTN stable enough to run unattended.
- Key functions/callbacks:
  - `ActionExecutor`: goal-level backoff `10_000 ms` after any action failure.
  - `AwaitPkt`: per-packet timeout failures as `pkt_timeout:<tag>` or `pkt_rejected:<tag>`.
  - `WaitUntil`: world-state timeout failures as `wait_until:<cond> timeout`.
  - `StoryWalkUpdate`: max `5` update attempts per task.
  - `StoryWaitStep`: resend every `5_000 ms`, fail after `90_000 ms`.
  - `StoryInteract`: wait up to `15` times outside instances or `60` times inside; after `3` raw-update fallbacks, block the task for `5 minutes`.
  - `QuestComplete`: per-quest attempt tracking, usually max `3`.
  - `QuestGrind`: declare stuck after `120_000 ms` without progress change.
  - `HTN.Combat`: blacklist bad targets for `30_000 ms`; support-mode disable happens once and stays latched.
  - `Revive` operator: resend cooldown `5_000 ms`, overall failure wall `30_000 ms`.
  - `WeepingVillage`: throttle repeated dungeon quest updates to once per `3_000 ms` for the same step.
  - `Proxy.Handler`: on HTN crash, records a report and backs the next HTN tick off to `5_000 ms`; `:htn_clear_blocks` clears most process-dictionary quest/instance retry state.
- Decision logic / priority / conditions: Most recovery state is stored in the process dictionary rather than in `GameState`, so a Rust port will need an equivalent per-connection transient state store for `:pending_attacks`, `:quest_complete_attempts`, `{:update_attempts, task_id}`, `{:update_blocked_until, task_id}`, `:registered_collections`, `:bought_skill_book_tiers`, and similar keys.
- Packet interactions: Recovery frequently depends on seeing or not seeing `1811`, `1804`, `1821`, `4004`, `215`, `319`, `608`, and shop/achievement result packets.
- Thresholds, constants, config values: See the bullet list above.
- How it interacts with other components: This recovery layer is spread across planner, actions, operators, `Proxy.Handler`, and `GameState`; it is not centralized in one subsystem.
