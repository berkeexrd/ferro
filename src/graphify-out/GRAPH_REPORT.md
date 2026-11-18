# Graph Report - src  (2026-11-18)

## Corpus Check
- 29 files · ~44,089 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 797 nodes · 2260 edges · 46 communities (32 shown, 14 thin omitted)
- Extraction: 87% EXTRACTED · 13% INFERRED · 0% AMBIGUOUS · INFERRED: 296 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Ui
- App
- Cmd
- app.rs
- theme.rs
- clean_err
- Icon
- egui
- Repo
- git.rs
- stage.rs
- github.rs
- clean_err
- overlay.rs
- Btn
- conflict.rs
- sys.rs
- i18n.rs
- graph.rs
- welcome.rs
- Ty
- Screen
- repo.rs
- icons.rs
- Id
- run
- Act
- palette.rs
- util.rs
- chrome.rs
- Modal
- Undo
- lists.rs
- ToastKind
- Kind

## God Nodes (most connected - your core abstractions)
1. `App` - 140 edges
2. `Cmd` - 62 edges
3. `Icon` - 56 edges
4. `semibold()` - 45 edges
5. `cr()` - 40 edges
6. `ok()` - 37 edges
7. `draw_left()` - 29 edges
8. `pointer()` - 28 edges
9. `run()` - 26 edges
10. `show()` - 26 edges

## Surprising Connections (you probably didn't know these)
- `App` --references--> `PaletteState`  [INFERRED]
  src/app.rs → src/screens/palette.rs
- `App` --references--> `SwitcherState`  [INFERRED]
  src/app.rs → src/screens/palette.rs
- `Act` --references--> `Cmd`  [INFERRED]
  src/screens/changes.rs → src/app.rs
- `commit_box()` --references--> `Cmd`  [INFERRED]
  src/screens/changes.rs → src/app.rs
- `side()` --references--> `Cmd`  [INFERRED]
  src/screens/changes.rs → src/app.rs

## Import Cycles
- None detected.

## Communities (46 total, 14 thin omitted)

### Community 0 - "Ui"
Cohesion: 0.07
Nodes (91): letter_of(), nav_rail(), picker(), status_bar(), title_bar(), is_en(), rt(), t() (+83 more)

### Community 1 - "App"
Cohesion: 0.10
Nodes (8): App, RedoSnap, staged_flags(), ResetSoft, Switch, Entry, parse_diff(), apply_visuals()

### Community 2 - "Cmd"
Cohesion: 0.04
Nodes (44): Cmd, CloneRepo, CloseRepo, Commit, CommitOp, CommitPush, CopyCommand, CopyPath (+36 more)

### Community 3 - "app.rs"
Cohesion: 0.13
Nodes (8): Acc, import_orbit_account(), Job, Menu, MenuItem, parse_github(), RepoRef, Settings

### Community 4 - "theme.rs"
Cohesion: 0.05
Nodes (47): color(), is_ident(), is_ident_start(), is_rust_char(), KW, Lang, CLike, Json (+39 more)

### Community 5 - "clean_err"
Cohesion: 0.16
Nodes (13): clean_err(), ok_env(), remote_url(), amend_message(), Blob, commit(), config_set(), delete_remote_branch() (+5 more)

### Community 6 - "Icon"
Cohesion: 0.06
Nodes (34): Icon, Alert, Branch, Check, Chev, ChevRight, Clone, Comment (+26 more)

### Community 8 - "Repo"
Cohesion: 0.12
Nodes (23): Block, FileView, Repo, all_lines(), build_patch(), compose_conflicts(), Conflict, DLine (+15 more)

### Community 9 - "git.rs"
Cohesion: 0.13
Nodes (23): activity_log(), auth_env(), auth_env_acc(), base_cmd(), COUNTER, KEYRING_SERVICE, LOG, LogEntry (+15 more)

### Community 10 - "stage.rs"
Cohesion: 0.14
Nodes (4): has_head(), paths_input(), stage_paths(), unstage_paths()

### Community 11 - "github.rs"
Cohesion: 0.21
Nodes (20): clear_token(), host_of(), set_token(), agent(), api_base(), gh_api(), gh_device_poll(), gh_device_start() (+12 more)

### Community 12 - "clean_err"
Cohesion: 0.31
Nodes (14): Out, run_full(), cherry_pick(), git_version(), merge(), op_continue(), op_result(), op_skip() (+6 more)

### Community 13 - "overlay.rs"
Cohesion: 0.24
Nodes (3): dialog(), menu_frame(), show_menu()

### Community 15 - "conflict.rs"
Cohesion: 0.18
Nodes (4): load_prs(), parse_iso(), Pr, PrState

### Community 16 - "sys.rs"
Cohesion: 0.15
Nodes (15): data_dir(), detect_editors(), exists_on_path(), Found, hidden(), load_state(), open_in_editor(), open_in_terminal() (+7 more)

### Community 17 - "i18n.rs"
Cohesion: 0.10
Nodes (18): build(), CUR, detect(), eframe::egui::Painter, fix_plural(), GEN, Lang, Auto (+10 more)

### Community 18 - "graph.rs"
Cohesion: 0.27
Nodes (9): CommitRow, Commit, search_commits(), compute(), Edge, GraphRow, Half, Lower (+1 more)

### Community 19 - "welcome.rs"
Cohesion: 0.18
Nodes (10): bottom_corners(), clone_panel(), dashed_rect(), default_root(), load_gh_repos(), LoginState, parse_percent(), repo_init() (+2 more)

### Community 20 - "Ty"
Cohesion: 0.17
Nodes (12): Ty, Body, Heading20, Label, Mono, MonoSm, MonoXs, Row (+4 more)

### Community 22 - "Screen"
Cohesion: 0.25
Nodes (8): Screen, Branches, Changes, Conflict, History, Log, Pr, Stash

### Community 23 - "repo.rs"
Cohesion: 0.10
Nodes (39): ok(), Branch, Branches, branches(), checkout(), commit_base(), commit_diff(), commit_files() (+31 more)

### Community 24 - "icons.rs"
Cohesion: 0.35
Nodes (5): svg!, paint(), paint_at(), source(), svg_of()

### Community 26 - "run"
Cohesion: 0.14
Nodes (16): run_undo(), run(), config_get(), Contributor, contributors(), diff_file(), discard(), DiscardItem (+8 more)

### Community 27 - "Act"
Cohesion: 0.29
Nodes (7): Act, Abort, Choose, Continue, File, Save, SetResult

### Community 29 - "palette.rs"
Cohesion: 0.28
Nodes (5): Item, matches(), palette_items(), PaletteState, SwitcherState

### Community 30 - "util.rs"
Cohesion: 0.27
Nodes (7): civil(), clock(), date_tr(), local_offset(), now(), rel(), rel_short()

### Community 34 - "chrome.rs"
Cohesion: 0.25
Nodes (4): sub(), win_btn(), eframe::egui::Response, RespT

### Community 36 - "Modal"
Cohesion: 0.33
Nodes (6): Modal, Confirm, FromCommit, Login, NewBranch, Settings

### Community 37 - "Undo"
Cohesion: 0.33
Nodes (6): OpEntry, Undo, Discard, IndexTree, None, ResetHard

### Community 38 - "lists.rs"
Cohesion: 0.73
Nodes (3): oplog(), page_header(), show()

### Community 39 - "ToastKind"
Cohesion: 0.40
Nodes (5): Toast, ToastKind, Err, Info, Ok

### Community 40 - "Kind"
Cohesion: 0.40
Nodes (5): Kind, Danger, Ghost, Primary, Secondary

## Knowledge Gaps
- **150 isolated node(s):** `Changes`, `History`, `Branches`, `Pr`, `Stash` (+145 more)
  These have ≤1 connection - possible missing edges. (Counts symbols only; 239 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **14 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `App` connect `App` to `Ui`, `chrome.rs`, `app.rs`, `Modal`, `Undo`, `theme.rs`, `ToastKind`, `Repo`, `lists.rs`, `egui`, `github.rs`, `overlay.rs`, `conflict.rs`, `sys.rs`, `welcome.rs`, `Screen`, `palette.rs`?**
  _High betweenness centrality (0.337) - this node is a cross-community bridge._
- **Are the 5 inferred relationships involving `App` (e.g. with `Tool` and `PaletteState`) actually correct?**
  _`App` has 5 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Changes`, `History`, `Branches` to the rest of the system?**
  _150 weakly-connected nodes found - possible missing edges._
- **Should `Ui` be split into smaller, more focused modules?**
  _Cohesion score 0.06692742285962625 - nodes in this community are weakly interconnected._
- **Why does `Cmd` connect `Cmd` to `Ui`, `App`, `app.rs`, `theme.rs`, `lists.rs`, `ToastKind`, `welcome.rs`, `Screen`, `palette.rs`?**
  _High betweenness centrality (0.121) - this node is a cross-community bridge._
- **Are the 11 inferred relationships involving `Cmd` (e.g. with `Act` and `commit_box()`) actually correct?**
  _`Cmd` has 11 INFERRED edges - model-reasoned connections that need verification._
- **Should `App` be split into smaller, more focused modules?**
  _Cohesion score 0.09894736842105263 - nodes in this community are weakly interconnected._