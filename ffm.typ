#set document(
  title: "FunnyFileManager Architecture",
  author: "FFM Development Team"
)

#set page(
  paper: "a4",
  margin: (x: 2cm, y: 2.5cm),
  header: [
    #set text(9pt)
    #h(1fr) FunnyFileManager Architecture Documentation #h(1fr)
  ],
  footer: [
    #set text(9pt)
    #h(1fr) #context counter(page).display("1 / 1", both: true) #h(1fr)
  ]
)

#set text(font: "New Computer Modern", size: 11pt)
#set heading(numbering: "1.1")
#set par(justify: true)

#show link: underline
#show raw: set text(size: 9pt)

// Title Page
#align(center)[
  #v(3cm)
  #text(size: 32pt, weight: "bold")[FunnyFileManager]
  #v(0.5cm)
  #text(size: 20pt)[Architecture Documentation]
  #v(1cm)
  #text(size: 14pt, style: "italic")[Terminal-based Dual-Panel File Manager]
  #v(0.5cm)
  #text(size: 12pt)[Inspired by Far Manager]
  #v(2cm)
  #line(length: 60%)
  #v(0.5cm)
  #text(size: 11pt)[Written in Rust using Ratatui TUI Framework]
  #v(0.3cm)
  #text(size: 10pt)[Version 0.1.0]
  #v(4cm)
]

#pagebreak()

// Table of Contents
#outline(title: "Table of Contents", indent: auto, depth: 3)

#pagebreak()

= Introduction

FunnyFileManager (FFM) is a terminal-based dual-panel file manager written in Rust. It provides a Far Manager-style user interface with keyboard-driven navigation and file operations.

== Key Features

- *Dual-panel layout* for efficient file operations between directories
- *Far Manager color scheme* (dark blue background, cyan text)
- *F-key commands* for all major operations
- *Quick search* with Alt+letter combinations
- *Bookmarks* for frequently visited directories
- *Built-in file viewer* for text files
- *External editor integration* with VS Code priority

== Technology Stack

#table(
  columns: (auto, 1fr),
  inset: 10pt,
  align: (left, left),
  stroke: 0.5pt,
  fill: (col, _) => if col == 0 { rgb("#e8e8f8") },
  [*Component*], [*Technology*],
  [Language], [Rust 2021 Edition],
  [TUI Framework], [Ratatui 0.26],
  [Terminal Backend], [Crossterm 0.27],
  [Testing], [Built-in + Tarpaulin for coverage],
)

#pagebreak()

= Project Structure

== File Organization

#align(center)[
  #box(stroke: 1pt + rgb("#4a90d9"), inset: 15pt, radius: 5pt, fill: rgb("#f8f8ff"))[
    ```
    FunnyFileManager/
    ├── src/
    │   ├── main.rs          # Entry point (thin wrapper)
    │   ├── lib.rs           # Public API, event loop
    │   ├── models.rs        # Core data structures
    │   ├── ui.rs            # TUI rendering
    │   ├── handlers.rs      # Event handlers
    │   ├── utils.rs         # Helper functions
    │   └── editor.rs        # Editor integration
    ├── tests/
    │   ├── editor_test.rs
    │   ├── handlers_test.rs
    │   └── navigation_features_test.rs
    ├── Cargo.toml
    └── CLAUDE.md
    ```
  ]
]

#v(1cm)

== Module Dependency Diagram

#align(center)[
  #box(stroke: 2pt + rgb("#333"), inset: 20pt, radius: 8pt)[
    #grid(
      columns: (1fr),
      gutter: 8pt,

      // Entry point
      align(center)[
        #box(fill: rgb("#90EE90"), stroke: 1pt, inset: 10pt, radius: 20pt)[
          #text(weight: "bold")[main.rs]
        ]
      ],

      align(center)[#text(16pt)[#sym.arrow.b]],

      // lib.rs
      align(center)[
        #box(fill: rgb("#87CEEB"), stroke: 1pt, inset: 10pt, radius: 5pt)[
          #text(weight: "bold")[lib.rs]
          #v(0.3em)
          #text(size: 9pt)[Event Loop & App Initialization]
        ]
      ],

      align(center)[
        #grid(
          columns: (1fr, 1fr, 1fr),
          gutter: 5pt,
          [#sym.arrow.bl], [#sym.arrow.b], [#sym.arrow.br],
        )
      ],

      // Middle layer
      grid(
        columns: (1fr, 1fr, 1fr),
        gutter: 10pt,
        align(center)[
          #box(fill: rgb("#DDA0DD"), stroke: 1pt, inset: 8pt, radius: 5pt)[
            #text(weight: "bold", size: 10pt)[ui.rs]
            #v(0.2em)
            #text(size: 8pt)[Rendering]
          ]
        ],
        align(center)[
          #box(fill: rgb("#FFB347"), stroke: 1pt, inset: 8pt, radius: 5pt)[
            #text(weight: "bold", size: 10pt)[handlers.rs]
            #v(0.2em)
            #text(size: 8pt)[Logic]
          ]
        ],
        align(center)[
          #box(fill: rgb("#98FB98"), stroke: 1pt, inset: 8pt, radius: 5pt)[
            #text(weight: "bold", size: 10pt)[editor.rs]
            #v(0.2em)
            #text(size: 8pt)[External]
          ]
        ],
      ),

      align(center)[
        #grid(
          columns: (1fr, 1fr),
          gutter: 30pt,
          [#sym.arrow.bl #h(1em) #sym.arrow.br], [#sym.arrow.b],
        )
      ],

      // Bottom layer
      grid(
        columns: (1fr, 1fr),
        gutter: 20pt,
        align(center)[
          #box(fill: rgb("#FFD700"), stroke: 1pt, inset: 8pt, radius: 5pt)[
            #text(weight: "bold", size: 10pt)[models.rs]
            #v(0.2em)
            #text(size: 8pt)[Data Structures]
          ]
        ],
        align(center)[
          #box(fill: rgb("#F0E68C"), stroke: 1pt, inset: 8pt, radius: 5pt)[
            #text(weight: "bold", size: 10pt)[utils.rs]
            #v(0.2em)
            #text(size: 8pt)[File Operations]
          ]
        ],
      ),
    )
  ]
]

#pagebreak()

= Core Data Structures

== Class Diagram

#align(center)[
  #box(stroke: 2pt + rgb("#000080"), inset: 15pt, radius: 5pt)[
    #grid(
      columns: (1fr, 1fr),
      gutter: 20pt,

      // App class
      [
        #box(fill: rgb("#e6f3ff"), stroke: 1pt, inset: 10pt, radius: 3pt, width: 100%)[
          #align(center)[#text(weight: "bold", size: 12pt)[App]]
          #line(length: 100%, stroke: 0.5pt)
          #text(size: 9pt)[
            + left_panel: Panel \
            + right_panel: Panel \
            + active_panel: PanelSide \
            + show_help: bool \
            + viewer: Option\<Viewer\> \
            + search: Option\<String\> \
            + command_line: Option\<String\> \
            + show_bookmarks: bool \
            + bookmarks: Vec\<Option\<PathBuf\>\> \
            + should_quit: bool \
            + status_message: String \
            + quick_search: Option\<String\>
          ]
          #line(length: 100%, stroke: 0.5pt)
          #text(size: 9pt)[
            + new() -> Result\<Self\> \
            + get_active_panel() -> &mut Panel \
            + get_inactive_panel() -> &mut Panel \
            + switch_panel()
          ]
        ]
      ],

      // Panel class
      [
        #box(fill: rgb("#fff3e6"), stroke: 1pt, inset: 10pt, radius: 3pt, width: 100%)[
          #align(center)[#text(weight: "bold", size: 12pt)[Panel]]
          #line(length: 100%, stroke: 0.5pt)
          #text(size: 9pt)[
            + path: PathBuf \
            + entries: Vec\<DirEntry\> \
            + state: ListState \
            + marked: BTreeSet\<String\> \
            + sort_mode: SortMode
          ]
          #line(length: 100%, stroke: 0.5pt)
          #text(size: 9pt)[
            + new(path) -> Result \
            + load_entries() -> Result \
            + next() / previous() \
            + enter() -> Result \
            + toggle_mark_selected() \
            + find_first_matching(prefix) \
            + find_next_matching(prefix) \
            + selected_path() -> Option
          ]
        ]
      ],
    )

    #v(1em)

    #grid(
      columns: (1fr, 1fr, 1fr),
      gutter: 15pt,

      // DirEntry
      box(fill: rgb("#e6ffe6"), stroke: 1pt, inset: 8pt, radius: 3pt)[
        #align(center)[#text(weight: "bold", size: 11pt)[DirEntry]]
        #line(length: 100%, stroke: 0.5pt)
        #text(size: 9pt)[
          + name: String \
          + is_dir: bool \
          + size: u64
        ]
      ],

      // SortMode
      box(fill: rgb("#ffe6f3"), stroke: 1pt, inset: 8pt, radius: 3pt)[
        #align(center)[#text(weight: "bold", size: 11pt)[SortMode]]
        #line(length: 100%, stroke: 0.5pt)
        #text(size: 9pt)[
          Name \
          Extension \
          Size
        ]
      ],

      // Viewer
      box(fill: rgb("#f3e6ff"), stroke: 1pt, inset: 8pt, radius: 3pt)[
        #align(center)[#text(weight: "bold", size: 11pt)[Viewer]]
        #line(length: 100%, stroke: 0.5pt)
        #text(size: 9pt)[
          + title: String \
          + lines: Vec\<String\> \
          + scroll: u16
        ]
      ],
    )
  ]
]

#pagebreak()

== Entity Relationships

#align(center)[
  #box(stroke: 1pt + rgb("#666"), inset: 20pt, radius: 8pt)[
    ```
    ┌─────────────────────────────────────────────────────────────────┐
    │                              App                                 │
    │  ┌───────────────┐                    ┌───────────────┐         │
    │  │  left_panel   │◄─────────┐   ┌────►│  right_panel  │         │
    │  │    :Panel     │          │   │     │    :Panel     │         │
    │  └───────┬───────┘          │   │     └───────┬───────┘         │
    │          │                  │   │             │                  │
    │          │ 1                │   │             │ 1                │
    │          │                  │   │             │                  │
    │          ▼                  │   │             ▼                  │
    │  ┌───────────────┐          │   │     ┌───────────────┐         │
    │  │   entries     │          │   │     │   entries     │         │
    │  │ Vec<DirEntry> │          │   │     │ Vec<DirEntry> │         │
    │  │     0..*      │     ┌────┴───┴───┐ │     0..*      │         │
    │  └───────────────┘     │active_panel│ └───────────────┘         │
    │                        │ :PanelSide │                            │
    │                        └────────────┘                            │
    │                                                                  │
    │  ┌─────────────────────────────────────────────────────────┐    │
    │  │              Overlays (mutually exclusive)               │    │
    │  │  ┌─────────┐ ┌─────────┐ ┌────────┐ ┌──────────────┐    │    │
    │  │  │show_help│ │ viewer  │ │ search │ │command_line  │    │    │
    │  │  │  :bool  │ │:Option  │ │:Option │ │   :Option    │    │    │
    │  │  └─────────┘ └─────────┘ └────────┘ └──────────────┘    │    │
    │  └─────────────────────────────────────────────────────────┘    │
    │                                                                  │
    │  bookmarks: [Option<PathBuf>; 10]                               │
    └─────────────────────────────────────────────────────────────────┘
    ```
  ]
]

#pagebreak()

== Panel State Machine

#align(center)[
  #box(stroke: 1pt + rgb("#333"), inset: 15pt, radius: 5pt)[
    ```
    ┌──────────────────────────────────────────────────────────────┐
    │                        Panel State                            │
    ├──────────────────────────────────────────────────────────────┤
    │                                                               │
    │   path: /home/user/documents                                  │
    │   ┌─────────────────────────────────────────────────────┐    │
    │   │ entries: Vec<DirEntry>                              │    │
    │   │  ┌────────────────────────────────────────────────┐ │    │
    │   │  │ [0] ".."        is_dir: true   size: 0         │ │    │
    │   │  │ [1] "folder1"   is_dir: true   size: 0         │ │    │
    │   │  │ [2] "file.txt"  is_dir: false  size: 1024      │◄┼────┼── selected
    │   │  │ [3] "image.png" is_dir: false  size: 2048      │ │    │
    │   │  └────────────────────────────────────────────────┘ │    │
    │   └─────────────────────────────────────────────────────┘    │
    │   state.selected() = Some(2)                                  │
    │   marked: {"folder1", "image.png"}                            │
    │   sort_mode: Name                                             │
    └──────────────────────────────────────────────────────────────┘

    Navigation State Transitions:
    ┌────────┐  Down   ┌────────┐  Down   ┌────────┐
    │ idx=0  │ ──────► │ idx=1  │ ──────► │ idx=2  │
    └────────┘         └────────┘         └────────┘
        ▲                                      │
        │              Up (wrap)               │
        └──────────────────────────────────────┘
    ```
  ]
]

#pagebreak()

= User Interface Architecture

== Screen Layout Diagram

#align(center)[
  #box(stroke: 3pt + rgb("#000080"), inset: 0pt, radius: 3pt, width: 95%)[
    // Header
    #box(fill: rgb("#000080"), width: 100%, inset: 6pt)[
      #text(fill: rgb("#00ffff"), size: 10pt, weight: "bold")[
        Far Manager style | Tab: Switch panels | F1 for help
      ]
    ]

    // Panels area
    #grid(
      columns: (1fr, 1fr),
      stroke: 1pt + rgb("#0000aa"),

      // Left Panel
      box(fill: rgb("#000080"), height: 180pt, width: 100%, inset: 8pt)[
        #box(fill: rgb("#00ffff"), width: 100%, inset: 4pt)[
          #text(fill: rgb("#000000"), size: 9pt, weight: "bold")[ /home/user ]
        ]
        #v(0.5em)
        #set text(size: 9pt)
        #text(fill: rgb("#ffffff"), weight: "bold")[#sym.arrow.r.double .. #h(3em) \<DIR\>] \
        #text(fill: rgb("#ffffff"), weight: "bold")[#h(0.3em) documents/ #h(1.5em) \<DIR\>] \
        #text(fill: rgb("#ffffff"), weight: "bold")[#h(0.3em) downloads/ #h(1.7em) \<DIR\>] \
        #box(fill: rgb("#00ffff"), inset: 2pt)[#text(fill: rgb("#000000"))[#h(0.3em) file.txt #h(2.2em) 1.5 KB]] \
        #text(fill: rgb("#00ffff"))[#h(0.3em) image.png #h(1.5em) 45 KB] \
        #text(fill: rgb("#00ffff"))[#h(0.3em) readme.md #h(1.2em) 2.1 KB]
      ],

      // Right Panel
      box(fill: rgb("#000080"), height: 180pt, width: 100%, inset: 8pt)[
        #box(fill: rgb("#0000aa"), width: 100%, inset: 4pt)[
          #text(fill: rgb("#8888ff"), size: 9pt)[ /tmp ]
        ]
        #v(0.5em)
        #set text(fill: rgb("#6688aa"), size: 9pt)
        #sym.arrow.r.double .. #h(3em) \<DIR\> \
        #h(0.3em) cache/ #h(2.5em) \<DIR\> \
        #h(0.3em) temp.dat #h(1.5em) 256 B
      ],
    )

    // Status bar
    #box(fill: rgb("#000080"), width: 100%, inset: 6pt)[
      #text(fill: rgb("#00ffff"), size: 9pt)[
        #sym.arrow.t#sym.arrow.b Navigate | Enter: Open | file.txt | 1.5 KB
      ]
    ]

    // Command bar
    #box(fill: rgb("#00ffff"), width: 100%, inset: 4pt)[
      #set text(size: 8pt)
      #box(fill: rgb("#000000"), inset: 2pt)[#text(fill: rgb("#00ffff"))[1]]#text(fill: rgb("#000000"))[Help ]
      #box(fill: rgb("#000000"), inset: 2pt)[#text(fill: rgb("#00ffff"))[2]]#text(fill: rgb("#000000"))[UserMn ]
      #box(fill: rgb("#000000"), inset: 2pt)[#text(fill: rgb("#00ffff"))[3]]#text(fill: rgb("#000000"))[View ]
      #box(fill: rgb("#000000"), inset: 2pt)[#text(fill: rgb("#00ffff"))[4]]#text(fill: rgb("#000000"))[Edit ]
      #box(fill: rgb("#000000"), inset: 2pt)[#text(fill: rgb("#00ffff"))[5]]#text(fill: rgb("#000000"))[Copy ]
      #box(fill: rgb("#000000"), inset: 2pt)[#text(fill: rgb("#00ffff"))[6]]#text(fill: rgb("#000000"))[RenMov ]
      #box(fill: rgb("#000000"), inset: 2pt)[#text(fill: rgb("#00ffff"))[7]]#text(fill: rgb("#000000"))[MkFold ]
      #box(fill: rgb("#000000"), inset: 2pt)[#text(fill: rgb("#00ffff"))[8]]#text(fill: rgb("#000000"))[Delete ]
      #box(fill: rgb("#000000"), inset: 2pt)[#text(fill: rgb("#00ffff"))[10]]#text(fill: rgb("#000000"))[Quit]
    ]
  ]
]

#v(0.5cm)

== Layout Constraints

#table(
  columns: (auto, auto, 1fr),
  inset: 8pt,
  stroke: 0.5pt,
  fill: (_, row) => if row == 0 { rgb("#4a90d9") },
  text(fill: white)[*Component*], text(fill: white)[*Height*], text(fill: white)[*Description*],
  [Header], [1 line], [Application title and keyboard hints],
  [Panels], [Remaining], [50/50 horizontal split],
  [Status Bar], [2 lines], [File info and quick search display],
  [Command Bar], [2 lines], [F-key function labels],
)

#pagebreak()

== Color Scheme

#table(
  columns: (auto, auto, 1fr),
  inset: 8pt,
  stroke: 0.5pt,
  fill: (_, row) => if row == 0 { rgb("#000080") },
  text(fill: white)[*Element*], text(fill: white)[*Color*], text(fill: white)[*RGB Value*],
  [Background], [Far Blue], [RGB(0, 0, 128)],
  [Active Border], [Cyan Bold], [RGB(0, 255, 255)],
  [Inactive Border], [Dark Blue], [RGB(0, 0, 139)],
  [Directories], [White Bold], [RGB(255, 255, 255)],
  [Files], [Cyan], [RGB(0, 255, 255)],
  [File Sizes], [Light Cyan], [RGB(224, 255, 255)],
  [Selection], [Black on Cyan], [BG: Cyan, FG: Black],
  [Marked Files], [Yellow], [RGB(255, 255, 0)],
)

#pagebreak()

= Event Loop and Key Handling

== Main Event Loop Flowchart

#align(center)[
  #box(stroke: 2pt + rgb("#333"), inset: 20pt, radius: 8pt)[
    #grid(
      columns: (1fr),
      gutter: 6pt,

      // Start
      align(center)[
        #box(fill: rgb("#90EE90"), stroke: 1pt, inset: 8pt, radius: 20pt)[
          *Start Application*
        ]
      ],

      align(center)[#sym.arrow.b],

      // Initialize
      align(center)[
        #box(fill: rgb("#e6f3ff"), stroke: 1pt, inset: 8pt, radius: 3pt)[
          Initialize Terminal & App State
        ]
      ],

      align(center)[#sym.arrow.b],

      // Main loop box
      align(center)[
        #box(stroke: 2pt + rgb("#0066cc"), inset: 10pt, radius: 5pt)[
          #text(weight: "bold")[Event Loop]
          #v(0.3em)

          #grid(
            columns: (1fr),
            gutter: 5pt,

            box(fill: rgb("#cce5ff"), stroke: 1pt, inset: 6pt, radius: 3pt)[
              Draw UI: terminal.draw(ui)
            ],

            align(center)[#sym.arrow.b],

            box(fill: rgb("#cce5ff"), stroke: 1pt, inset: 6pt, radius: 3pt)[
              Read Event: event::read()
            ],

            align(center)[#sym.arrow.b],

            box(fill: rgb("#fff3cd"), stroke: 1pt, inset: 6pt, radius: 3pt)[
              #rotate(-45deg, reflow: true)[Overlay Active?]
            ],

            grid(
              columns: (1fr, 1fr),
              [Yes #sym.arrow.r handle_overlay_keys()],
              [No #sym.arrow.b],
            ),

            box(fill: rgb("#ffe6cc"), stroke: 1pt, inset: 6pt, radius: 3pt)[
              Route Input (navigation, F-keys, etc.)
            ],
          )
        ]
      ],

      align(center)[#sym.arrow.b],

      // Quit check
      align(center)[
        #box(fill: rgb("#fff3cd"), stroke: 1pt, inset: 6pt, radius: 3pt)[
          should_quit == true?
        ]
      ],

      grid(
        columns: (1fr, 1fr),
        gutter: 20pt,
        align(center)[No #sym.arrow.tl Loop],
        align(center)[
          Yes #sym.arrow.r
          #box(fill: rgb("#ffcccc"), stroke: 1pt, inset: 6pt, radius: 15pt)[
            *Exit*
          ]
        ],
      ),
    )
  ]
]

#pagebreak()

== Key Event Priority

#table(
  columns: (auto, auto, 1fr),
  inset: 8pt,
  stroke: 0.5pt,
  fill: (_, row) => if row == 0 { rgb("#4a90d9") },
  text(fill: white)[*Priority*], text(fill: white)[*Handler*], text(fill: white)[*Keys Handled*],
  [1], [Overlay Keys], [Viewer, Help, Search, Bookmarks, Command Line],
  [2], [Quit Keys], [Q, Ctrl+C, F10],
  [3], [Navigation], [Up, Down, Enter, Tab, Left, Right],
  [4], [Function Keys], [F1-F8 for file operations],
  [5], [Marking], [Insert, Space for file selection],
  [6], [Quick Search], [Alt+letter for fast navigation],
  [7], [Other Keys], [Clears quick search state],
)

== Overlay State Machine

#align(center)[
  #box(stroke: 1pt + rgb("#333"), inset: 15pt, radius: 5pt)[
    ```
    ┌─────────────────────────────────────────────────────────────┐
    │                    Overlay State Machine                     │
    ├─────────────────────────────────────────────────────────────┤
    │                                                              │
    │                      ┌──────────┐                            │
    │            ┌─────────│  NORMAL  │─────────┐                  │
    │            │         └────┬─────┘         │                  │
    │            │              │               │                  │
    │     F1     │       F3     │      F2       │     Ctrl+;       │
    │            ▼              ▼               ▼                  ▼
    │      ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
    │      │  HELP    │   │  VIEWER  │   │BOOKMARKS │   │ COMMAND  │
    │      │  Overlay │   │  Overlay │   │  Overlay │   │   LINE   │
    │      └────┬─────┘   └────┬─────┘   └────┬─────┘   └────┬─────┘
    │           │              │              │              │      │
    │     Esc/F1│        Esc/F3│         Esc/F2              Esc    │
    │           │        Enter │         0-9: Go             Enter  │
    │           │              │       Ctrl+0-9: Set               │
    │           └──────────────┴──────────────┴──────────────┘      │
    │                              │                                │
    │                              ▼                                │
    │                         ┌──────────┐                          │
    │                         │  NORMAL  │                          │
    │                         └──────────┘                          │
    └─────────────────────────────────────────────────────────────┘
    ```
  ]
]

#pagebreak()

== Keyboard Shortcuts Reference

=== Navigation Keys

#table(
  columns: (auto, 1fr),
  inset: 8pt,
  stroke: 0.5pt,
  fill: (_, row) => if row == 0 { rgb("#e8e8e8") },
  [*Key*], [*Action*],
  [#sym.arrow.t / #sym.arrow.b], [Move selection up/down (with wraparound)],
  [#sym.arrow.l / #sym.arrow.r], [Switch between panels],
  [Tab], [Toggle between panels],
  [Enter], [Enter directory or go back (..)],
  [Home], [Jump to first item],
  [End], [Jump to last item],
  [PgUp / PgDn], [Page navigation],
)

=== Function Keys

#table(
  columns: (auto, auto, 1fr),
  inset: 8pt,
  stroke: 0.5pt,
  fill: (_, row) => if row == 0 { rgb("#008080") },
  text(fill: white)[*Key*], text(fill: white)[*Command*], text(fill: white)[*Description*],
  [F1], [Help], [Toggle help overlay],
  [F2], [Bookmarks], [Show bookmark manager],
  [F3], [View], [View file in built-in viewer],
  [F4], [Edit], [Open file in external editor],
  [F5], [Copy], [Copy to other panel],
  [F6], [Move], [Move to other panel],
  [F7], [Mkdir], [Create new directory],
  [F8], [Delete], [Delete selected item],
  [F10], [Quit], [Exit application],
)

=== Quick Search Keys

#table(
  columns: (auto, 1fr),
  inset: 8pt,
  stroke: 0.5pt,
  fill: (_, row) => if row == 0 { rgb("#e8e8e8") },
  [*Key*], [*Action*],
  [Alt+letter], [Add letter to search query, jump to first match],
  [Backspace], [Remove last character or clear search],
  [Esc], [Clear quick search],
)

#pagebreak()

= File Operation Handlers

== Handler Overview

#align(center)[
  #box(stroke: 1pt + rgb("#333"), inset: 15pt, radius: 5pt, width: 90%)[
    ```
    handlers.rs
    ├── handle_overlay_keys()  ─► Overlay input processing
    ├── handle_view()          ─► F3: File viewing
    ├── handle_copy()          ─► F5: Copy operation
    ├── handle_move()          ─► F6: Move operation
    ├── handle_mkdir()         ─► F7: Directory creation
    ├── handle_delete()        ─► F8: Item deletion
    └── execute_command()      ─► Command line handler
    ```
  ]
]

== Copy Operation Sequence Diagram

#align(center)[
  #box(stroke: 1pt + rgb("#333"), inset: 15pt, radius: 5pt)[
    ```
    handle_copy(app)
           │
           ▼
    ┌──────────────────────┐
    │ Get source from      │
    │ active_panel         │
    │ (marked or selected) │
    └──────────┬───────────┘
               │
               ▼
    ┌──────────────────────┐
    │ Get destination from │
    │ inactive_panel.path  │
    └──────────┬───────────┘
               │
               ▼
    ┌──────────────────────┐
    │ unique_destination() │
    │ (avoid conflicts)    │
    └──────────┬───────────┘
               │
               ▼
         ┌───────────┐
         │  is_dir?  │
         └─────┬─────┘
          Yes  │  No
          ┌────┴────┐
          ▼         ▼
    ┌──────────┐ ┌──────────┐
    │copy_dir_ │ │fs::copy()│
    │   all()  │ │  (file)  │
    │(recursive)│ │          │
    └─────┬────┘ └────┬─────┘
          │           │
          └─────┬─────┘
                ▼
    ┌──────────────────────┐
    │  refresh_panels()    │
    │  Update status msg   │
    └──────────────────────┘
    ```
  ]
]

#pagebreak()

== Move Operation Sequence Diagram

#align(center)[
  #box(stroke: 1pt + rgb("#333"), inset: 15pt, radius: 5pt)[
    ```
    handle_move(app)
           │
           ▼
    ┌──────────────────────┐
    │ Get source and       │
    │ destination paths    │
    └──────────┬───────────┘
               │
               ▼
    ┌──────────────────────┐
    │  Try fs::rename()    │
    │  (atomic, fast)      │
    └──────────┬───────────┘
               │
         ┌─────┴─────┐
         │ Success?  │
         └─────┬─────┘
          Yes  │  No (cross-device)
          │    │
          │    ▼
          │  ┌──────────────────────┐
          │  │ Copy to destination  │
          │  │ (copy_dir_all or     │
          │  │  fs::copy)           │
          │  └──────────┬───────────┘
          │             │
          │             ▼
          │  ┌──────────────────────┐
          │  │ Delete source        │
          │  │ (remove_dir_all or   │
          │  │  remove_file)        │
          │  └──────────┬───────────┘
          │             │
          └──────┬──────┘
                 ▼
    ┌──────────────────────┐
    │  refresh_panels()    │
    └──────────────────────┘
    ```
  ]
]

#pagebreak()

== Command Line Commands

#table(
  columns: (auto, auto, 1fr),
  inset: 8pt,
  stroke: 0.5pt,
  fill: (_, row) => if row == 0 { rgb("#e8e8e8") },
  [*Command*], [*Arguments*], [*Action*],
  [`cd`], [`<path>`], [Change directory (active panel)],
  [`pwd`], [], [Show current path in status],
  [`sort`], [`name|ext|size`], [Change sort mode],
  [`bookmark`], [`set|go <0-9>`], [Manage bookmarks],
  [`mark`], [`all`], [Mark all files],
  [`unmark`], [], [Clear all marks],
  [`help`], [], [Show help overlay],
  [`quit`], [], [Exit application],
)

#pagebreak()

= Utility Functions

== utils.rs Overview

#table(
  columns: (auto, 1fr),
  inset: 8pt,
  stroke: 0.5pt,
  fill: (_, row) => if row == 0 { rgb("#e8e8e8") },
  [*Function*], [*Purpose*],
  [`format_size(bytes)`], [Convert bytes to human-readable format],
  [`unique_destination(base, name)`], [Generate unique filename if exists],
  [`copy_dir_all(src, dst)`], [Recursive directory copy],
  [`refresh_panels(app)`], [Reload both panels],
)

== format_size() Logic

#align(center)[
  #box(stroke: 1pt + rgb("#333"), inset: 15pt, radius: 5pt)[
    ```
    format_size(bytes: u64) → String

    bytes < 1024           →  "{bytes} B"
    bytes < 1,048,576      →  "{bytes/1024:.2} KB"
    bytes < 1,073,741,824  →  "{bytes/1048576:.2} MB"
    bytes >= 1,073,741,824 →  "{bytes/1073741824:.2} GB"

    Examples:
      512        →  "512 B"
      2048       →  "2.00 KB"
      5242880    →  "5.00 MB"
      2147483648 →  "2.00 GB"
    ```
  ]
]

== unique_destination() Algorithm

#align(center)[
  #box(stroke: 1pt + rgb("#333"), inset: 15pt, radius: 5pt)[
    ```
    unique_destination(base: &Path, file_name: &OsStr) → PathBuf

    Input: base=/target, file_name="test.txt"

    Check: /target/test.txt exists?
      No  → return /target/test.txt
      Yes → try /target/test_1.txt
              → try /target/test_2.txt
              → try /target/test_3.txt
              → ... until unique found

    Edge cases handled:
      • Dotfiles: ".env" → ".env_1"
      • No extension: "Makefile" → "Makefile_1"
      • Multiple dots: "file.tar.gz" → "file.tar_1.gz"
    ```
  ]
]

#pagebreak()

== Recursive copy_dir_all() Algorithm

#align(center)[
  #box(stroke: 1pt + rgb("#333"), inset: 15pt, radius: 5pt)[
    ```
    copy_dir_all(src, dst)
           │
           ▼
    ┌──────────────────────┐
    │ fs::create_dir_all   │
    │      (dst)           │
    └──────────┬───────────┘
               │
               ▼
    ┌──────────────────────┐
    │ for entry in         │◄────────────┐
    │   fs::read_dir(src)  │             │
    └──────────┬───────────┘             │
               │                         │
               ▼                         │
         ┌─────────────┐                 │
         │  is_dir?    │                 │
         └──────┬──────┘                 │
           Yes  │  No                    │
           │    │                        │
           ▼    ▼                        │
    ┌──────────┐ ┌──────────┐            │
    │copy_dir_ │ │fs::copy()│            │
    │all(entry)│ │ (file)   │            │
    │(recurse) │ │          │            │
    └──────────┘ └──────────┘            │
           │         │                   │
           └────┬────┘                   │
                │                        │
                └────────────────────────┘
                    (next entry)
    ```
  ]
]

#pagebreak()

= Editor Integration

== Editor Selection Strategy

#align(center)[
  #box(stroke: 1pt + rgb("#333"), inset: 15pt, radius: 5pt)[
    ```
    get_editor_command() → (program, args)

    ┌─────────────────────────────────────────────┐
    │ 1. Check VS Code availability               │
    │    Run: code --version                      │
    │    If success → return ("code", ["-w","-n"])│
    └─────────────────────┬───────────────────────┘
                          │ Failed
                          ▼
    ┌─────────────────────────────────────────────┐
    │ 2. Check EDITOR environment variable        │
    │    Parse: "vim -c 'set number'"             │
    │    → program: "vim"                         │
    │    → args: ["-c", "set number"]             │
    └─────────────────────┬───────────────────────┘
                          │ Not set
                          ▼
    ┌─────────────────────────────────────────────┐
    │ 3. Fallback to nano                         │
    │    return ("nano", [])                      │
    └─────────────────────────────────────────────┘
    ```
  ]
]

== handle_edit() Sequence

#align(center)[
  #box(stroke: 1pt + rgb("#333"), inset: 15pt, radius: 5pt)[
    ```
    handle_edit(terminal, app)
           │
           ▼
    ┌──────────────────────┐
    │ Validate selection   │
    │ (not dir, not None)  │
    └──────────┬───────────┘
               │
               ▼
    ┌──────────────────────┐
    │ Restore terminal     │
    │ • Leave raw mode     │
    │ • Leave alt screen   │
    └──────────┬───────────┘
               │
               ▼
    ┌──────────────────────┐
    │ Spawn editor process │
    │ • Wait for exit      │
    └──────────┬───────────┘
               │
               ▼
    ┌──────────────────────┐
    │ Restore terminal     │
    │ • Enter raw mode     │
    │ • Enter alt screen   │
    └──────────┬───────────┘
               │
               ▼
    ┌──────────────────────┐
    │ refresh_panels()     │
    │ Update status        │
    └──────────────────────┘
    ```
  ]
]

#pagebreak()

= Data Flow Patterns

== Panel Navigation Flow

#align(center)[
  #box(stroke: 1pt + rgb("#333"), inset: 15pt, radius: 5pt)[
    ```
    User Input (Down key)
           │
           ▼
    ┌──────────────────────┐
    │ Event handler        │
    │ (run_app_with_event) │
    └──────────┬───────────┘
               │
               ▼
    ┌──────────────────────┐
    │ panel.next()         │
    └──────────┬───────────┘
               │
               ▼
    ┌──────────────────────┐
    │ Update ListState     │
    │ .selected()          │
    └──────────┬───────────┘
               │
               ▼
    ┌──────────────────────┐
    │ Next frame: ui()     │
    │ render_panel uses    │
    │ ListState to         │
    │ highlight selection  │
    └──────────────────────┘
    ```
  ]
]

== Quick Search Flow

#align(center)[
  #box(stroke: 1pt + rgb("#333"), inset: 15pt, radius: 5pt)[
    ```
    Alt+A pressed
         │
         ▼
    quick_search = Some("a")
         │
         ▼
    find_first_matching("a")
         │
         ▼
    Selection moves to first "a*" entry
         │
         ▼
    Status bar: "Quick search: a_"
         │
         ▼
    Alt+B pressed
         │
         ▼
    quick_search = Some("ab")
         │
         ▼
    find_first_matching("ab")
         │
         ├── Found → Selection moves
         │
         └── Not found → Try just "b"
                            │
                            ▼
                   find_first_matching("b")
    ```
  ]
]

#pagebreak()

= Error Handling

== Error Handling Strategy

#table(
  columns: (auto, 1fr, 1fr),
  inset: 8pt,
  stroke: 0.5pt,
  fill: (_, row) => if row == 0 { rgb("#e8e8e8") },
  [*Operation*], [*Error Type*], [*Handling*],
  [Directory access], [`io::Error`], [Show in status, continue],
  [File view (non-UTF8)], [`io::Error`], [Display error message],
  [Copy/Move/Delete], [`io::Error`], [Status message, refresh panels],
  [Editor launch], [Process error], [Status: "Failed to launch"],
  [Panel reload], [`io::Error`], [Attempt refresh anyway],
)

== Error Flow Diagram

#align(center)[
  #box(stroke: 1pt + rgb("#333"), inset: 15pt, radius: 5pt)[
    ```
    Operation (e.g., handle_copy)
           │
           ▼
    ┌──────────────────────┐
    │ Perform I/O          │
    └──────────┬───────────┘
               │
         ┌─────┴─────┐
         │ Result?   │
         └─────┬─────┘
          Ok   │  Err(e)
          │    │
          │    ▼
          │  ┌──────────────────────┐
          │  │ app.status_message = │
          │  │ format!("Error: {e}")│
          │  └──────────┬───────────┘
          │             │
          └──────┬──────┘
                 │
                 ▼
    ┌──────────────────────┐
    │ refresh_panels()     │
    │ (attempt recovery)   │
    └──────────────────────┘
                 │
                 ▼
    ┌──────────────────────┐
    │ Continue execution   │
    │ (non-fatal)          │
    └──────────────────────┘
    ```
  ]
]

#pagebreak()

= Testing Architecture

== Test Pyramid

#align(center)[
  #box(stroke: 1pt + rgb("#333"), inset: 15pt, radius: 5pt, width: 90%)[
    ```
    ┌─────────────────────────────────────────────────────────┐
    │                     Test Pyramid                         │
    ├─────────────────────────────────────────────────────────┤
    │                                                          │
    │                    ┌───────────┐                         │
    │                    │Integration│                         │
    │                    │   Tests   │                         │
    │                    └─────┬─────┘                         │
    │               ┌──────────┴──────────┐                    │
    │               │   Handler Tests     │                    │
    │               │ (multi-step ops)    │                    │
    │               └──────────┬──────────┘                    │
    │          ┌───────────────┴───────────────┐               │
    │          │         Unit Tests            │               │
    │          │  (data structures, utils)     │               │
    │          └───────────────────────────────┘               │
    │                                                          │
    └─────────────────────────────────────────────────────────┘
    ```
  ]
]

== Test Infrastructure

#table(
  columns: (auto, 1fr),
  inset: 8pt,
  stroke: 0.5pt,
  fill: (_, row) => if row == 0 { rgb("#e8e8e8") },
  [*Component*], [*Purpose*],
  [`WritableTestBackend`], [Custom ratatui backend for testing],
  [`tempfile::TempDir`], [Temporary directories for FS tests],
  [`FORCE_RENAME_FAILURE`], [Simulate cross-device moves],
  [`FORCE_COPY_DIR_ALL_STEP`], [Inject errors at specific steps],
  [`FORCE_REFRESH_ERROR`], [Test panel refresh failures],
)

== Mutation Testing Points

The codebase includes specific injection points for mutation testing:

```rust
// In copy_dir_all()
#[cfg(any(test, tarpaulin))]
if FORCE_COPY_DIR_ALL_STEP.load(Ordering::SeqCst) == STEP {
    return Err(io::Error::new(...));
}

// In handle_move()
#[cfg(any(test, tarpaulin))]
static FORCE_RENAME_FAILURE: AtomicBool = AtomicBool::new(false);
```

#pagebreak()

= Architectural Decisions

== Key Design Choices

=== 1. Dual-Panel Architecture

*Rationale*: Enables intuitive source-target operations without dialogs.

#box(stroke: 1pt + rgb("#4a90d9"), inset: 10pt, radius: 3pt, width: 100%)[
  *Benefits:*
  - Visual directory comparison
  - One-key file operations (F5, F6)
  - Independent navigation state per panel
]

=== 2. Overlay Stack Pattern

*Rationale*: Modal interactions without complex window management.

#box(stroke: 1pt + rgb("#4a90d9"), inset: 10pt, radius: 3pt, width: 100%)[
  *Benefits:*
  - Simple state machine (one overlay at a time)
  - Consistent Escape behavior
  - Easy to add new overlays
]

=== 3. Selection Preservation

*Rationale*: Reduces user disorientation after operations.

#box(stroke: 1pt + rgb("#4a90d9"), inset: 10pt, radius: 3pt, width: 100%)[
  *Implementation:*
  - Save selected filename before reload
  - Find filename after reload
  - Fall back to first item if not found
]

=== 4. Rename-with-Fallback for Move

*Rationale*: Handle cross-filesystem moves correctly.

#box(stroke: 1pt + rgb("#4a90d9"), inset: 10pt, radius: 3pt, width: 100%)[
  *Strategy:*
  1. Try atomic `fs::rename()` first (fast path)
  2. On failure, copy then delete (slow path)
  3. User sees same result regardless of filesystems
]

=== 5. Unique Destination Naming

*Rationale*: Never silently overwrite existing files.

#box(stroke: 1pt + rgb("#4a90d9"), inset: 10pt, radius: 3pt, width: 100%)[
  *Strategy:*
  - Check if destination exists
  - Append `_1`, `_2`, etc. to base name
  - Preserve original extension
]

#pagebreak()

= Extension Guide

== Adding New Features

=== New F-Key Command

```rust
// In lib.rs, match key.code block:
KeyCode::F(9) => {
    handle_new_feature(&mut app)?;
}
```

=== New Overlay

```rust
// In models.rs, add to App:
pub new_overlay: Option<NewOverlayData>,

// In handlers.rs, handle_overlay_keys():
if let Some(ref mut overlay) = app.new_overlay {
    // Handle overlay-specific keys
    return true;
}

// In ui.rs, render_overlays():
if app.new_overlay.is_some() {
    render_new_overlay(frame, app);
}
```

=== New Sort Mode

```rust
// In models.rs, SortMode enum:
pub enum SortMode {
    Name,
    Extension,
    Size,
    Date,  // New mode
}

// Add comparison logic in Panel::load_entries()
```

== Configuration Points

#table(
  columns: (auto, auto, 1fr),
  inset: 8pt,
  stroke: 0.5pt,
  fill: (_, row) => if row == 0 { rgb("#e8e8e8") },
  [*Setting*], [*Location*], [*Current Value*],
  [Initial directory], [`lib.rs`], [`env::current_dir()`],
  [Color scheme], [`ui.rs`], [Far Manager blue],
  [Editor preference], [`editor.rs`], [VS Code > EDITOR > nano],
  [Bookmark slots], [`models.rs`], [10 slots (0-9)],
  [Debounce time], [`lib.rs`], [250ms (coverage mode)],
)

#pagebreak()

= Conclusion

FunnyFileManager demonstrates a well-architected terminal application with:

#box(stroke: 2pt + rgb("#4a90d9"), inset: 15pt, radius: 5pt, width: 100%)[
  == Architectural Highlights

  #table(
    columns: (auto, 1fr),
    inset: 8pt,
    stroke: none,
    [*Separation of Concerns*], [Distinct modules for models, UI, handlers, utilities],
    [*Immutable Rendering*], [UI functions don't mutate state],
    [*Comprehensive Error Handling*], [Non-fatal errors, graceful recovery],
    [*Far Manager UX*], [Familiar hotkeys and color scheme],
    [*Testable Design*], [Injection points for mutation testing],
    [*Terminal Safety*], [Proper mode restoration for external programs],
  )
]

#v(1cm)

#align(center)[
  #box(stroke: 1pt + rgb("#666"), inset: 20pt, radius: 10pt, fill: rgb("#f0f8ff"))[
    #text(size: 16pt, weight: "bold")[
      FunnyFileManager
    ]
    #v(0.3cm)
    #text(size: 11pt)[
      Modern Terminal File Manager in Rust
    ]
    #v(0.3cm)
    #text(size: 10pt, style: "italic")[
      Built with Ratatui + Crossterm
    ]
  ]
]
