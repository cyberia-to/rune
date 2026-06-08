# computation digraphs

[← specification index](README.md)

---

digraphs compose two characters: the first (family) carries the atomic semantic; the second (variant) determines the shape within that family.

## variant axis (second character)

| char | name | shape within family |
|------|------|---------------------|
| `-` | hep | minimal — the bare, degenerate form |
| `=` | tis | explicit — with binding or comparison |
| `*` | tar | generic — wildcard, broadcast, polymorphic |
| `+` | lus | augmented — more arguments, larger arity |
| `/` | fas | structural — scoped, path-aware |
| `.` | dot | composed — transform, pipeline |
| `:` | col | paired — two-branch, two-way |
| `>` | gar | flowing — sequential composition |
| `~` | sig | null — absent, optional, empty |
| `^` | ket | meta — abstract, lifted |
| `&` | pam | combined — trace, accumulate |
| `_` | cab | reversed — alternative, mirror |
| `%` | cen | rich — multi-part, structured |
| (doubled) | — | canonical — the prototype of the family |

## the 25 digraphs

| digraph | family × variant | what it makes | arity |
|---------|------------------|---------------|-------|
| `\|=` bartis | core × explicit | gate (with sample) | 2 |
| `\|*` bartar | core × generic | wet gate | 2 |
| `\|-` barhep | core × minimal | trap (no sample) | 1 |
| `\|%` barcen | core × rich | door (multi-arm) | 1 |
| `=/` tisfas | bind × structural | let binding | 3 |
| `=.` tisdot | bind × composed | rebind subject slot | 3 |
| `=>` tisgar | bind × flowing | compose right against left | 2 |
| `?:` wutcol | test × paired | if-then-else | 3 |
| `?-` wuthep | test × minimal | switch by tag | 2 |
| `?=` wuttis | test × explicit | pattern match with binding | 2 |
| `?~` wutsig | test × null | null test | 3 |
| `:-` colhep | cell × minimal | pair | 2 |
| `:+` collus | cell × augmented | triple | 3 |
| `:_` colcab | cell × reversed | flipped pair | 2 |
| `^-` kethep | cast × minimal | cast to explicit mold | 2 |
| `^+` ketlus | cast × augmented | cast to value's mold | 2 |
| `.*` dottar | eval × generic | evaluate formula against subject | 2 |
| `.+` dotlus | eval × augmented | increment | 1 |
| `.^` dotket | eval × meta | scry — read from graph | 2 |
| `~&` sigpam | hint × combined | trace | 2 |
| `~_` sigcab | hint × reversed | hint pair | 2 |
| `/+` faslus | build × augmented | import library | 1 |
| `/-` fashep | build × minimal | import types only | 1 |
| `++` luslus | arm × canonical | arm definition | 2 |
| `!!` zapzap | crash × canonical | crash | 0 |

every digraph decomposes: `?-` = test-minimal = simplest test (switch). `\|*` = core-generic = wet gate. `.^` = eval-meta = scry over the graph layer.

fixed arity per digraph. precedence disappears. tall and flat forms parse identically.

digraphs do not collide with cybermark sigils: sigils stand alone (`#name`, `@alice`, `~truth`); digraphs are exactly two symbols with no whitespace (`\|=`, `?:`, `=/`).
