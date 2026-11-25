//! Purpose: The Language servers dialog's screens cell for cell, as the
//! layout tests expect them: the list at 80×24 and at every height where a
//! row gives way, and the F3 view.

pub(super) const LIST_80X24: &[&str] = &[
    "╔══════════════════════════════ Language servers ══════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║   bat                 [ ] batch-language-server      .bat .cmd       missing ║",
    "║   css                 [x] vscode-css-language-server .css              ready ║",
    "║   html                [x] vscode-html-language-serv… .html .htm        ready ║",
    "║   java                [x] jdtls                      .java              auto ║",
    "║   javascript          [x] typescript-language-server .js .mjs .cjs      auto ║",
    "║   javascriptreact     [x] typescript-language-server .jsx               auto ║",
    "║   json                [x] vscode-json-language-serv… .json .jsonc      ready ║",
    "║ ▸ python              [x] pylsp                      .py .pyi          ready ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ python  [enabled]  .py .pyi                                                  ║",
    "║   Command:          pylsp                                                    ║",
    "║   Arguments:        (none)                                                   ║",
    "║   Status:           installed in FFM cache                                   ║",
    "║   Executable:       /home/bob/.config/ffm/lsp/7c1e/venv/bin/python           ║",
    "║   Launch arguments: -m pylsp                                                 ║",
    "║   Auto-install:     {\"kind\":\"python\",\"package\":\"python-lsp-server[pyflakes,p ║",
    "║                     ycodestyle]\",\"module\":\"pylsp\"}                           ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ No changes                                                                   ║",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═════════════════════╝",
    "1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12",
];

pub(super) const LIST_80X16: &[&str] = &[
    "╔══════════════════════════════ Language servers ══════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║   javascript          [x] typescript-language-server .js .mjs .cjs      auto ║",
    "║   javascriptreact     [x] typescript-language-server .jsx               auto ║",
    "║   json                [x] vscode-json-language-serv… .json .jsonc      ready ║",
    "║ ▸ python              [x] pylsp                      .py .pyi          ready ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ python  [enabled]  .py .pyi                                                  ║",
    "║   Command:          pylsp                                                    ║",
    "║   Arguments:        (none)                                                   ║",
    "║ … F3 shows everything                                                        ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ No changes                                                                   ║",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═════════════════════╝",
    "1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12",
];

pub(super) const LIST_80X15: &[&str] = &[
    "╔══════════════════════════════ Language servers ══════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║   bat                 [ ] batch-language-server      .bat .cmd       missing ║",
    "║   css                 [x] vscode-css-language-server .css              ready ║",
    "║   html                [x] vscode-html-language-serv… .html .htm        ready ║",
    "║   java                [x] jdtls                      .java              auto ║",
    "║   javascript          [x] typescript-language-server .js .mjs .cjs      auto ║",
    "║   javascriptreact     [x] typescript-language-server .jsx               auto ║",
    "║   json                [x] vscode-json-language-serv… .json .jsonc      ready ║",
    "║ ▸ python              [x] pylsp                      .py .pyi          ready ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ F3 shows details                                                             ║",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═════════════════════╝",
    "1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12",
];

pub(super) const LIST_60X12: &[&str] = &[
    "╔════════════════════ Language servers ════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml            ║",
    "╟──────────────────────────────────────────────────────────╢",
    "║   java                [x] jdtls    .java            auto ║",
    "║   javascript          [x] typescr… .js .mjs .cjs    auto ║",
    "║   javascriptreact     [x] typescr… .jsx             auto ║",
    "║   json                [x] vscode-… .json .jsonc    ready ║",
    "║ ▸ python              [x] pylsp    .py .pyi        ready ║",
    "╟──────────────────────────────────────────────────────────╢",
    "║ F3 shows details                                         ║",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═╝",
    "1    2Save3View4Edit5Che…6    7Add 8Res…9    10Qu…11   12",
];

pub(super) const LIST_4X3: &[&str] = &["╔══╗", "╚ E╝", "1234"];

pub(super) const VIEW_80X24: &[&str] = &[
    "╔═══════════════════════════ Language servers: all ════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "║ Availability is what FFM would start; it is not a process list.              ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ bat  [disabled]  .bat .cmd                                                   ║",
    "║   Command:          batch-language-server                                    ║",
    "║   Arguments:        --stdio                                                  ║",
    "║   Status:           not installed; auto-install disabled                     ║",
    "║   Auto-install:     {\"kind\":\"npm\",\"packages\":[\"@jcs-pr/batch-language-server ║",
    "║                     \"],\"package\":\"@jcs-pr/batch-language-server\",\"binary\":\"b ║",
    "║                     atch-language-server\"}                                   ║",
    "║                                                                              ║",
    "║ css  [enabled]  .css                                                         ║",
    "║   Command:          vscode-css-language-server                               ║",
    "║   Arguments:        --stdio                                                  ║",
    "║   Status:           found configured executable                              ║",
    "║   Executable:       /usr/local/bin/vscode-css-language-server                ║",
    "║   Launch arguments: --stdio                                                  ║",
    "║   Auto-install:     {\"kind\":\"npm\",\"packages\":[\"vscode-langservers-extracted\" ║",
    "║                     ],\"package\":\"vscode-langservers-extracted\",\"binary\":\"vsc ║",
    "║                     ode-css-language-server\"}                                ║",
    "║                                                                              ║",
    "╚ Lines 1-18 of 110  Up Down PgUp PgDn Home End  F3/Esc List ══════════════════╝",
    "1      2      3List  4      5Check 6      7      8      9     10Back11    12",
];

pub(super) const VIEW_80X4: &[&str] = &[
    "╔═══════════════════════════ Language servers: all ════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "╚ Lines 0-0 of 110  Up Down PgUp PgDn Home End  F3/Esc List ═══════════════════╝",
    "1      2      3List  4      5Check 6      7      8      9     10Back11    12",
];

pub(super) const LIST_80X3: &[&str] = &[
    "╔══════════════════════════════ Language servers ══════════════════════════════╗",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═════════════════════╝",
    "1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12",
];

pub(super) const LIST_80X4: &[&str] = &[
    "╔══════════════════════════════ Language servers ══════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═════════════════════╝",
    "1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12",
];

pub(super) const LIST_80X5: &[&str] = &[
    "╔══════════════════════════════ Language servers ══════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "║ F3 shows details                                                             ║",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═════════════════════╝",
    "1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12",
];

pub(super) const LIST_80X6: &[&str] = &[
    "╔══════════════════════════════ Language servers ══════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ F3 shows details                                                             ║",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═════════════════════╝",
    "1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12",
];

pub(super) const LIST_80X7: &[&str] = &[
    "╔══════════════════════════════ Language servers ══════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ F3 shows details                                                             ║",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═════════════════════╝",
    "1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12",
];

pub(super) const LIST_80X8: &[&str] = &[
    "╔══════════════════════════════ Language servers ══════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ ▸ python              [x] pylsp                      .py .pyi          ready ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ F3 shows details                                                             ║",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═════════════════════╝",
    "1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12",
];

pub(super) const VIEW_80X6: &[&str] = &[
    "╔═══════════════════════════ Language servers: all ════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "║ Availability is what FFM would start; it is not a process list.              ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "╚ Lines 0-0 of 110  Up Down PgUp PgDn Home End  F3/Esc List ═══════════════════╝",
    "1      2      3List  4      5Check 6      7      8      9     10Back11    12",
];

pub(super) const VIEW_80X7: &[&str] = &[
    "╔═══════════════════════════ Language servers: all ════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "║ Availability is what FFM would start; it is not a process list.              ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ bat  [disabled]  .bat .cmd                                                   ║",
    "╚ Lines 1-1 of 110  Up Down PgUp PgDn Home End  F3/Esc List ═══════════════════╝",
    "1      2      3List  4      5Check 6      7      8      9     10Back11    12",
];
