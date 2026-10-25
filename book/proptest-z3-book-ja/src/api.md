# 付録: API 一覧

proptest-z3 0.1.0 の公開 API を、シグネチャと意味論(特にスコープ・ステップ・評価順との関係)まで含めて列挙します。
本文の補足として、また LLM にテストを書かせるときのコンテキストとして使うことを想定しています。

## A.1 読む前に: 2種類の番号

proptest-z3 には、穴を指す番号が2種類あります。混ぜると誤ったテストになります。

| | 型 | 並び | 使う場所 |
| --- | --- | --- | --- |
| 評価順の番号 | `usize`(変数名は `i`, `k`, `j`) | 評価順(A.2) | 制約の中(`Binds`、`rules`) |
| テキスト順の番号 | `Hole(pub usize)` | 出力テキストに現れる順 | 解いた後(`Solved`) |

- 制約の中で `i < k` は「`i` が `k` より先に評価される」を意味します。テキスト上の前後ではありません。
- 制約の中から `Hole` を得る公開 API はありません。穴の位置が必要なら `binds.mark(i, ..)` で印を付け、解いた後に `s.marks()` で `Hole` として受け取ります。

## A.2 ステップ・評価順・スコープの規則

### ステップ

- ステップは「定義が見えるようになる単位」で、言語の文に当たります。
- ステップを作るのは `Skeleton::steps(..)` / `Frag::scope(..)` / `Frag::boundary(..)` の3つだけです。どれも、渡された各要素を1ステップとして閉じます。
- `frag!` / `Frag::concat` / `Frag::join` はステップを作りません。連結した断片は、外側で閉じられたときに同じ1ステップになります。
- `Frag::scope(..)` / `Frag::boundary(..)` の中の穴は中のステップに属し、外側の文のステップには属しません。

### 評価順

評価順は次の規則で決まります。

1. ステップはテキスト順に並びます。
2. 1つのステップの中では、
   - 先: `Use` の穴と、そのステップに含まれる入れ子スコープ内の穴(テキスト順)
   - 後: そのステップ自身の `Def` と `Kill` の穴(テキスト順)

帰結:

- `let x = x` の右辺 `x`(`Use`)は、同じ文の左辺 `x`(`Def`)より先に評価されるので、それを指しません。
- `del x`(`Kill`)は、同じ文の `Use` より後に評価されます。
- `fn f { .. call f .. }` のように、文の `Def` とその文に含まれるスコープを並べると、スコープ内の穴はすべて `f` の定義より先に評価されます。**そのため、既定では再帰呼び出しは見えません。** 境界(`Frag::boundary`)でも同じです。
- 境界の中身(クロージャの本体)は、書かれた位置で評価されたものとして扱われます(その場で呼ばれたのと同じ)。境界の中の `Kill` は、境界の後の参照にも効きます。

### スコープ

- `Frag::scope(steps)` が入れ子のスコープを1つ作ります。スコープは字句的(静的)な入れ子だけです。
- スコープ内の `Def` は、そのスコープとさらに内側のスコープからだけ見えます。外側と兄弟スコープからは見えません。
- 外側の `Def` は内側から見えます。内側で同名を定義すると、内側ではそちらが優先されます(シャドーイング)。
- スコープは z3 に渡されません。Rust 側の `bool`(`Binds::encloses`)として、制約を組み立てるときに候補を絞るのに使われます。

### 境界(クロージャ・関数本体)

- `Frag::boundary(steps)` は、**境界の印が付いたスコープ**を作ります。クロージャや関数の本体に使います。
- 見える範囲(`encloses`、`visible`、`target` など)は `Frag::scope` と全く同じです。境界は名前解決の結果を変えません。
- 違いは、参照が境界を何個越えて定義に届くかを数えられることだけです(`Binds::boundaries_between`)。これを使って「キャプチャ」(`captured`)を判定します。
- 普通のスコープ(`Frag::scope`)は越えても数えません。境界の中のブロック、ブロックの中の境界は、どちらも自由に入れ子にできます。

### 各 API がスコープを跨ぐか

凡例:

- **字句的**: 外側の定義は内側から見える。内側・兄弟スコープの定義は見えない
- **全体**: スコープを見ない。骨格全体のすべての穴が対象
- **跨がない**: 同じステップの中だけ(同じステップの穴は必ず同じスコープにある)
- **—**: スコープと関係しない

| API | スコープ | 補足 |
| --- | --- | --- |
| `Binds::defs` / `uses` / `kills` / `refs` / `of` | 全体 | 名前空間と役割で絞るだけ |
| `Binds::same_step` | 跨がない | 入れ子スコープ内の穴と、そのスコープを含む外側の文の穴は別ステップ |
| `Binds::encloses` | 字句的 | これ自体がスコープの判定。境界も普通のスコープとして扱う |
| `Binds::boundaries_between` | 字句的 | `inner` を囲み `outer` を囲まない境界の数。普通のスコープは数えない |
| `reaching_defs` | 字句的 | 評価順で前にある、同じ名前空間の `Def` のうち、`i` を囲むスコープのもの(境界も越えて見える) |
| `kills_between` | **全体** | `Kill` はスコープも境界も見ない(下の注意) |
| `visible` | 字句的 + `Kill` は全体 | `reaching_defs` と `kills_between` の組み合わせ |
| `killed` | 字句的 + `Kill` は全体 | 同上 |
| `resolves_to` | 字句的 + `Kill` は全体 | `Solved::target` と同じ規則を制約として書いたもの |
| `captured` | 字句的(境界を数える) | 指す定義と参照の間に境界が1つ以上ある |
| `bound` / `unbound` | 字句的 + `Kill` は全体 | `refs`(全体)の各要素に `visible` を適用 |
| `defined_in_same_step` | 跨がない | 評価順の前後は問わない |
| `distinct_defs` | **全体** | 内側スコープの同名定義(シャドーイング)や、兄弟スコープどうしの同名も「重複」に数える |
| `Solved::target` | 字句的 + `Kill` は全体 | `visible` と同じ規則を、解いた後の具体的な名前に適用 |
| `Solved::events` | 全体 | 骨格全体の評価順 |
| `Binds::mark` / `observe` / `require` | — | |
| `NameSpec` / `Names` | — | 候補は名前空間ごとで、スコープごとではない |

**`Kill` はスコープを跨ぎます。** 定義 `k` から参照 `i` までの間(評価順)に、同じ名前空間・同じ名前の `Kill` が1つでもあれば、`k` は `i` から見えなくなります。その `Kill` がどのスコープにあるかは問いません。

- 内側のスコープで `del a` すると、外側の `a` も消えます。内側に同名の `a` があって、`del a` がそちらを指していても同じです。
- 閉じた兄弟スコープの中の `del a` も、後の `a` の参照に効きます。
- `Kill` の後に同名の `Def` があれば、その定義は新しく見えます。ただし、見える範囲はその `Def` のスコープの中だけです。

**`distinct_defs` はスコープを跨ぎます。** `!distinct_defs(binds, S)` は「どこかに同名の定義の組がある」という意味で、同じスコープでの再代入に限りません。同じスコープの中での再定義だけを禁じたい・許したい場合は、`binds.encloses` と `binds.same_step` で定義の組を絞り、`constraint!` で自分で書きます。

### 例

この付録の例では、本文と同じ小さな言語を使います(`def x` は定義、`set x y` は `y` を使って `x` を定義、`print x` は使用、`del x` は削除、`{ .. }` はスコープ)。名前空間は `VAR` だけで、候補は `a`, `b`, `c` です。

```text
0| def a
1| set b a
2| { def a print a }
3| del a
4| print b
```

| テキスト順 `Hole` | 穴 | 役割 | スコープ | 評価順 `i` |
| --- | --- | --- | --- | --- |
| `Hole(0)` | 0行目 `a` | `Def` | トップ | 0 |
| `Hole(1)` | 1行目 `b` | `Def` | トップ | 2 |
| `Hole(2)` | 1行目 `a` | `Use` | トップ | 1 |
| `Hole(3)` | 2行目 1つ目の `a` | `Def` | 内側 | 3 |
| `Hole(4)` | 2行目 2つ目の `a` | `Use` | 内側 | 4 |
| `Hole(5)` | 3行目 `a` | `Kill` | トップ | 5 |
| `Hole(6)` | 4行目 `b` | `Use` | トップ | 6 |

- 1行目は `Use` の `a`(`i = 1`)が `Def` の `b`(`i = 2`)より先に評価されます。
- `reaching_defs(4)` は `{0, 2, 3}`。`Solved::target(Hole(4))` は内側の `Hole(3)` です(同名で生きている定義のうち、評価順で最後のもの)。
- `reaching_defs(5)` は `{0, 2}`。内側の `Hole(3)` はトップの `Kill` を囲まないので入りません。`target(Hole(5))` は `Hole(0)` です。
- `print b`(`i = 6`)は `Hole(1)` を指します。間の `Kill` は名前が `a` なので効きません。
- `distinct_defs(binds, VAR)` は偽です。`Hole(0)` と `Hole(3)` がどちらも `a` だからです(スコープは違いますが、全体で数えます)。
- 3行目が `{ del a }` でも結果は同じです(`Kill` はスコープの中にあっても外側の `a` を消します)。
- 2行目を `{ def a del a }` に変えると、内側の `del a` は内側の `a` を指しますが、外側の `a`(`Hole(0)`)も同時に消えます。その後のトップレベルでの `a` の参照は削除済み(`killed`)になります。

### 例: 境界

`[ .. ]` を境界(`Frag::boundary`)、`{ .. }` を普通のスコープとします。

```text
0| def a
1| [ def b [ print a print b ] { print b } ]
```

| テキスト順 `Hole` | 穴 | 役割 | 囲むもの(内側から) | 評価順 `i` |
| --- | --- | --- | --- | --- |
| `Hole(0)` | 0行目 `a` | `Def` | なし | 0 |
| `Hole(1)` | `def b` の `b` | `Def` | 境界1 | 1 |
| `Hole(2)` | `print a` の `a` | `Use` | 境界2, 境界1 | 2 |
| `Hole(3)` | 境界2の中の `print b` の `b` | `Use` | 境界2, 境界1 | 3 |
| `Hole(4)` | `{ }` の中の `print b` の `b` | `Use` | スコープ, 境界1 | 4 |

- `Hole(2)` は `Hole(0)` を指します。`boundaries_between(0, 2) = 2`(境界1と境界2)なので `captured` は真です。境界を2つ越えて届くので、外側のクロージャが内側へキャプチャを中継していることになります。
- `Hole(3)` は `Hole(1)` を指します。`boundaries_between(1, 3) = 1`(境界2だけ。境界1は定義も囲む)なので `captured` は真です。
- `Hole(4)` も `Hole(1)` を指しますが、間にあるのは普通のスコープだけなので `boundaries_between(1, 4) = 0`、`captured` は偽です。

## A.3 モジュールと prelude

`use proptest_z3::prelude::*;` で次が入ります。

| 由来 | 項目 |
| --- | --- |
| proptest-z3 | `frag!`, `Binds`, `Fill`, `FillExt`, `Filled`, `Frag`, `Given`, `Hint`, `Hole`, `IntoFrag`, `NameSpec`, `Names`, `Role`, `Skeleton`, `Solved`, `Space`, `Text`, `label_stats` |
| `proptest_z3::rules::*` | `visible`, `bound`, `unbound`, `defined_in_same_step`, `killed`, `distinct_defs`, `resolves_to`, `captured`, `reaching_defs`, `kills_between` |
| z3 | `z3::ast::{Bool, Int}` |
| z3-toolkit | `z3_toolkit::prelude::*`(`constraint!`、`Failure` など) |

prelude に入らず、クレート直下にあるもの: `FilledHole`, `Label`, `LabelStat`, `take_label_stats`, `take_rejections`。

```rust
use proptest::prelude::*;
use proptest_z3::prelude::*;
use proptest_z3::{take_label_stats, take_rejections}; // 必要なときだけ
```

## A.4 形を作る

### `Space`(名前空間)

`#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]`。中身の文字列で区別されます(同じ文字列の `Space` は同じ名前空間)。

| シグネチャ | 内容 |
| --- | --- |
| `const fn new(name: &'static str) -> Space` | `const` として定義する: `const VAR: Space = Space::new("var");` |
| `fn name(self) -> &'static str` | 表示や panic のメッセージに使われる |
| `fn hole(self, role: Role) -> impl Strategy<Value = Frag>` | 穴1つの `Frag`。ヒントは proptest が決める |
| `fn def(self) -> impl Strategy<Value = Frag>` | `hole(Role::Def)` |
| `fn refer(self) -> impl Strategy<Value = Frag>` | `hole(Role::Use)` |
| `fn kill(self) -> impl Strategy<Value = Frag>` | `hole(Role::Kill)` |

名前空間どうしは独立です。`VAR` の `a` と `FUNC` の `a` は無関係で、どの規則も同じ名前空間の穴だけを見ます。

### `Role`(穴の役割)

| 値 | 意味 |
| --- | --- |
| `Role::Def` | 名前を定義する |
| `Role::Use` | 名前を参照する |
| `Role::Kill` | 名前を参照し、同時に削除する。以後、同名の定義はすべて見えない(A.2) |

- `fn is_ref(self) -> bool`: `Use` か `Kill` なら真。この2つを**参照**と呼びます。
- `Kill` は参照でもあるので、`bound` を課すと「生きている名前を削除する」になります。

### `Hint`(希望の名前)

`pub struct Hint(pub u16)`。0〜1 の固定小数点数で、「候補の何番目の名前が欲しいか」を表します。`Arbitrary` を実装しています。

- `fn index(self, size: usize) -> usize`: `floor(size × h / 65536)`。`size == 0` なら panic。
- 制約とぶつからない限り守られます。縮小では 0 に寄るので、名前は候補の先頭(A.5 の通り辞書順で最小)に寄ります。
- 普段は `Space::def()` などが自動で作るので、直接使うのは `Frag::hole` を手で呼ぶときだけです。

### `Frag`(組み立て中の断片)

`#[derive(Debug, Clone, Default)]`。`Frag::default()` は空の断片です。

| シグネチャ | 内容 | ステップ・スコープ |
| --- | --- | --- |
| `fn hole(space: Space, role: Role, hint: Hint) -> Frag` | 穴1つ | 作らない |
| `fn text(s: impl Into<String>) -> Frag` | テキストだけ | 作らない |
| `fn concat(parts: impl IntoIterator<Item = Frag>) -> Frag` | 順に連結 | 作らない |
| `fn join(sep: impl IntoFrag + Clone, parts: impl IntoIterator<Item = impl IntoFrag>) -> Frag` | 区切り付きで連結 | 作らない |
| `fn scope(steps: impl IntoIterator<Item = Frag>) -> Frag` | 入れ子のスコープ。各要素を1ステップとして閉じる | ステップとスコープを作る |
| `fn boundary(steps: impl IntoIterator<Item = Frag>) -> Frag` | 境界の印が付いた入れ子のスコープ(クロージャ・関数本体)。見える範囲は `scope` と同じ | ステップとスコープ(境界)を作る |

- `Frag::scope` / `Frag::boundary` の戻り値はまだ `Frag` です。外側の文の一部として `frag![..]` に置き、外側の `Skeleton::steps` か `Frag::scope` / `Frag::boundary` で閉じます。
- 中の要素に区切り(空白・改行)を付けるのは利用者です: `Frag::scope(body.into_iter().map(|s| frag![s, " "]))`。
- 括弧などの見た目も利用者が付けます。たとえばブロックとクロージャを作り分けるなら:

```rust
fn block(stmts: Vec<Frag>) -> Frag {
    frag!["{ ", Frag::scope(stmts.into_iter().map(|s| frag![s, " "])), "}"]
}

fn closure(stmts: Vec<Frag>) -> Frag {
    frag!["[ ", Frag::boundary(stmts.into_iter().map(|s| frag![s, " "])), "]"]
}
```

### `IntoFrag` と `frag!`

`frag![e1, e2, ..]` は `Frag::concat([e1.into_frag(), e2.into_frag(), ..])` です。末尾のカンマは省略可。

`IntoFrag` の実装:

| 型 | 変換 |
| --- | --- |
| `Frag`, `&Frag` | そのまま(`&Frag` は clone) |
| `Vec<T: IntoFrag>` | 要素を連結 |
| `Option<T: IntoFrag>` | `None` は空 |
| `(T1, T2)`, `(T1, T2, T3)`, `(T1, T2, T3, T4)` | 要素を連結 |
| `&str`, `String`, `&String`, `char`, `bool` | テキスト |
| 整数(`i8`〜`i128`, `isize`, `u8`〜`u128`, `usize`)、`f32`, `f64` | `to_string()` したテキスト |
| `Text<T: Display>` | `to_string()` したテキスト |

利用者の型は `IntoFrag` を直接実装するか、`Display` を実装して `Text(x)` で包みます。`Display` には穴を入れられません(文字列にした時点で穴の情報が消えます)。

### `Skeleton`(完成した骨格)

| シグネチャ | 内容 |
| --- | --- |
| `fn steps(steps: impl IntoIterator<Item = Frag>) -> Skeleton` | 各要素を1ステップとして閉じ、骨格を完成させる。`Skeleton` を作る唯一の方法 |
| `fn holes(&self) -> usize` | 穴の数 |
| `fn solve(&self, names: &Names, fill: Fill, rule: impl Fn(&Binds) -> Bool) -> Result<Solved, Failure>` | proptest を通さずに直接解く(A.10) |

`fill` は `Strategy<Value = Skeleton>` に対してだけ使えるので、ステップの閉じ忘れは型エラーになります。

典型的な形:

```rust
fn program() -> impl Strategy<Value = Skeleton> {
    prop::collection::vec(stmt(), 1..8)
        .prop_map(|stmts| Skeleton::steps(stmts.into_iter().map(|s| frag![s, "\n"])))
}
```

## A.5 名前の候補

### `NameSpec`(候補を作る strategy)

`Strategy<Value = Names>` を実装しています。`Clone`。

| シグネチャ | 内容 |
| --- | --- |
| `fn new() -> NameSpec` | 空 |
| `fn space(self, space: Space, regex: &str, count: usize) -> NameSpec` | `regex` に合う、互いに異なる `count` 個の名前。候補は**辞書順**に並ぶ |
| `fn space_with(self, space: Space, names: BoxedStrategy<Vec<String>>) -> NameSpec` | 候補の strategy を直接渡す。並び順はそのまま候補の番号になる |

- 名前の候補も proptest が作るので、縮小で名前自体も短くなります。
- `regex` に合う文字列が `count` 個未満だと、候補を作れません。
- 候補が少ないほど同じ名前が繰り返し使われ、名前解決にとって難しい入力になります。ただし「互いに異なる」系の制約は解けにくくなります(例: 候補3個で定義4個に `distinct_defs` は unsat)。

panic する条件: 不正な正規表現 / 同じ `Space` を2回登録 / `space_with` の候補に空文字列か重複がある。

### `Names`(具体的な候補)

| シグネチャ | 内容 |
| --- | --- |
| `fn default() -> Names` | 空 |
| `fn with(self, space: Space, names: impl IntoIterator<Item = impl Into<String>>) -> Names` | 候補を手で設定する(`Skeleton::solve` を直接呼ぶ単体テスト向け) |
| `fn get(&self, space: Space) -> &[String]` | 候補の列。未登録なら空 |

## A.6 制約を書く: `Binds`

`fill` / `solve` に渡す規則 `rule: Fn(&Binds) -> Bool` は、`Binds` を受け取り z3 の `Bool` を返します。規則は1回解くごとに(縮小の各段も含めて)呼ばれるので、副作用を持たせないでください(`mark` などの記録は解くごとに作り直されます)。

`Binds` の番号はすべて評価順です(A.1)。

### 公開フィールド

| フィールド | 型 | 内容 |
| --- | --- | --- |
| `role` | `Vec<Role>` | 各穴の役割 |
| `space` | `Vec<Space>` | 各穴の名前空間 |
| `name` | `Vec<Int>` | 各穴に入る名前。z3 の整数変数で、値は候補の番号(`0..候補数`)。範囲の制約は自動で付く |

名前の等しさは `binds.name[p] == binds.name[q]`(`constraint!` の中)で書きます。名前の文字列は制約からは見えません。

### メソッド

| シグネチャ | 戻り値 | スコープ | 内容 |
| --- | --- | --- | --- |
| `fn len(&self) -> usize` | | | 穴の数 |
| `fn is_empty(&self) -> bool` | | | |
| `fn of(&self, space: Space, role: Role) -> impl Iterator<Item = usize>` | 評価順 | 全体 | その名前空間・役割の穴 |
| `fn defs(&self, space: Space)` | 同上 | 全体 | `of(space, Def)` |
| `fn uses(&self, space: Space)` | 同上 | 全体 | `of(space, Use)` |
| `fn kills(&self, space: Space)` | 同上 | 全体 | `of(space, Kill)` |
| `fn refs(&self, space: Space)` | 同上 | 全体 | `Use` と `Kill` |
| `fn same_step(&self, p: usize, q: usize) -> bool` | Rust の `bool` | 跨がない | 同じステップか |
| `fn encloses(&self, outer: usize, inner: usize) -> bool` | Rust の `bool` | 字句的 | `outer` の属するスコープが `inner` を囲むか。`outer` がトップレベルなら常に真。同じスコープなら真。境界も普通のスコープとして扱う |
| `fn boundaries_between(&self, outer: usize, inner: usize) -> usize` | Rust の `usize` | 字句的 | `inner` を囲み、`outer` を囲まない境界(`Frag::boundary`)の数。普通のスコープは数えない。`encloses(outer, inner)` が真のときに使う(定義 `outer` から参照 `inner` まで越える境界の数) |
| `fn mark<L: Label>(&self, i: usize, kind: L, broken: Bool) -> Bool` | `broken` をそのまま返す | — | A.8 |
| `fn observe<L: Label>(&self, kind: L, cond: Bool)` | | — | A.8 |
| `fn require<L: Label>(&self, label: L, constraint: Bool)` | 返さない | — | A.8 |

`same_step`、`encloses`、`boundaries_between` は生成時に決まる情報なので、z3 の式ではなく Rust の値です。`filter` や `if` で候補を絞るのに使います。

## A.7 制約を書く: 規則(`proptest_z3::rules`)

すべて `&Binds` を受け取り、z3 の `Bool` を返します。`i` は評価順の番号です。

| シグネチャ | スコープ | 真になる条件 |
| --- | --- | --- |
| `visible(binds: &Binds, i: usize) -> Bool` | 字句的 + `Kill` は全体 | 参照 `i` が、見えていて削除されていない定義を指す。すなわち `reaching_defs(i)` のどれか `k` について、`name[k] == name[i]` かつ `k` と `i` の間に同名の `Kill` がない |
| `killed(binds: &Binds, i: usize) -> Bool` | 同上 | 参照 `i` の名前は、見える範囲で定義されたが削除されている。かつ `!visible(i)` |
| `bound(binds: &Binds, space: Space) -> Bool` | 同上 | `space` のすべての参照が `visible` |
| `unbound(binds: &Binds, space: Space, n: usize) -> Bool` | 同上 | `space` の参照のうち、ちょうど `n` 個が `!visible` |
| `defined_in_same_step(binds: &Binds, i: usize) -> Bool` | 跨がない | `i` と同じステップに、同じ名前空間・同じ名前の `Def` がある(評価順の前後は問わない) |
| `distinct_defs(binds: &Binds, space: Space) -> Bool` | **全体** | `space` の `Def` の名前が互いに異なる。`Def` が0〜1個なら真 |
| `resolves_to(binds: &Binds, i: usize, k: usize) -> Bool` | 字句的 + `Kill` は全体 | 参照 `i` が定義 `k` を指す。すなわち `k` が `reaching_defs(i)` に含まれ、`name[k] == name[i]`、`reaching_defs(i)` のうち `k` より後に同名がなく、`k` と `i` の間に同名の `Kill` がない。`k` が `reaching_defs(i)` に含まれなければ定数の偽 |
| `captured(binds: &Binds, i: usize) -> Bool` | 字句的(境界を数える) | 参照 `i` が、`i` を囲む境界の外にある定義を指す(キャプチャ)。すなわち `boundaries_between(k, i) > 0` を満たす `reaching_defs(i)` のどれか `k` について `resolves_to(i, k)` |

制約の材料になる関数(イテレータを返し、z3 の式ではありません):

| シグネチャ | スコープ | 内容 |
| --- | --- | --- |
| `reaching_defs(binds: &Binds, i: usize) -> impl Iterator<Item = usize>` | 字句的 | `i` が指し得る定義の候補。`k < i`、`Def`、`i` と同じ名前空間、`encloses(k, i)` を満たす `k`(評価順) |
| `kills_between(binds: &Binds, k: usize, i: usize) -> impl Iterator<Item = usize>` | **全体** | `k < j < i`(評価順で厳密に間)、`Kill`、`i` と同じ名前空間の `j` |

`resolves_to(i, k)` は `Solved::target` を制約にしたものです。解いた後に `target(i の穴) == Some(k の穴)` となるのは、`resolves_to(i, k)` が真のときだけです。`visible(i)` は「`reaching_defs(i)` のどれかに `resolves_to`」と同値です。

### 参照の3状態

参照 `i` は次のどれか1つになります。`!visible` は「削除済み」と「未定義」の和です。

| 状態 | 式 |
| --- | --- |
| 見えている | `visible(binds, i)` |
| 削除済み | `killed(binds, i)` |
| 未定義(どこにも見える定義がない) | `(!visible(binds, i)).given([!killed(binds, i)])` |

「未定義」には、外側から見えない定義(閉じたスコープの中・兄弟スコープの中)しかない場合も含まれます。「スコープ外の定義を指している」と「どこにも定義がない」を区別したいときは、`binds.defs(..)` を `encloses` で絞り、`constraint!` で自分で書きます。

### よく使う形

```rust
// 正常系
bound(binds, VAR)

// 再定義を含む正常系(スコープを跨いだ同名も含む)
constraint!(bound(binds, VAR), !distinct_defs(binds, VAR))

// 異常系: 参照がちょうど1つ見えない。その穴に印を付ける
constraint!(exactly(1) { for i in binds.refs(VAR) {
    binds.mark(i, Bug::Unbound, !visible(binds, i))
} })

// 自己参照(`let x = x`)
constraint!(all { !visible(binds, i), defined_in_same_step(binds, i) })

// 定義 k が使われている(未使用の定義の検出に。`!` を付けると未使用)
constraint!(any { for i in binds.refs(VAR) { resolves_to(binds, i, k) } })

// キャプチャされた参照に印を付ける(分類なので制約にはしない)
for i in binds.refs(VAR) {
    binds.mark(i, Captured, captured(binds, i));
}

// 境界を2つ以上越えるキャプチャ(外側のクロージャが中継する)が少なくとも1つある
fn relayed(binds: &Binds, i: usize) -> Bool {
    let cases: Vec<Bool> = reaching_defs(binds, i)
        .filter(|&k| binds.boundaries_between(k, i) >= 2)
        .map(|k| resolves_to(binds, i, k))
        .collect();
    Bool::or(&cases)
}
constraint!(bound(binds, VAR), any { for i in binds.refs(VAR) { relayed(binds, i) } })
```

境界の数で条件を変えたいときは、`captured` を使わずに、`relayed` のように `reaching_defs` を `boundaries_between` で絞って `resolves_to` を並べます。

## A.8 印・分類・名前付き制約(ラベル)

### `Label`

`Copy + Debug + PartialEq + Send + Sync + 'static` な型はすべて `Label` です(包括実装)。利用者の列挙体や `&'static str` を使います。

- ラベルの**名前**は、`&'static str` ならその文字列、それ以外は `Debug` の出力です。統計(`label_stats`)と棄却の集計(`take_rejections`)は、この名前で数えます。型の違う2つのラベルでも、名前が同じなら同じものとして数えます。
- `marks::<L>()` / `labels::<L>()` は型 `L` で絞ります。

### 記録する(`Binds` のメソッド)

| シグネチャ | 制約になるか | 内容 |
| --- | --- | --- |
| `mark(i, kind, broken: Bool) -> Bool` | 戻り値を制約に入れたときだけ | 穴 `i` が `kind` として壊れていると記録する。解いた後、`broken` が真なら `s.marks()` に `(Hole, kind)` が出る |
| `observe(kind, cond: Bool)` | ならない | 解いた後に `cond` が真か調べるだけ。生成の分布を変えずに分類できる |
| `require(label, constraint: Bool)` | なる(必須) | 名前付きの制約。解けないとき、unsat core に `label` の名前が出る。値を返さないので `constraint!` の中には書けない。規則の本体で呼ぶ |

- `mark` は `broken` をそのまま返します。返り値を `exactly(1) { .. }` などの中に置いて初めて制約になります。置かなければ穴付きの `observe` と同じです。
- `mark` の真偽は、解いた後のモデルで `broken` を評価して決まります。1つの穴に複数の `mark` が真になることがあります(違反の条件が重なっている場合)。
- `rule` の戻り値と名前の範囲の制約には名前がないので、unsat core に出ません。名前のない制約と矛盾したときは、名前付きの側だけが理由に出ます。理由が1つの名前だけでも、その制約単体で満たせないとは限りません。棄却の原因を調べたい制約はすべて `require` に移し、戻り値は `Bool::from_bool(true)` にします。
- 同じ制約を `require` しつつ `rule` の戻り値にも入れると、名前のない側だけで矛盾を説明できるので、そのラベルは理由に出なくなります。
- 同じラベルで複数回 `require` すると、1つの名前付き制約としてまとまります(ループの中で穴ごとに呼んでも、理由には1回だけ出ます)。

```rust
fn rule(binds: &Binds) -> Bool {
    binds.require("self_ref", constraint!(any { for i in binds.uses(VAR) { defined_in_same_step(binds, i) } }));
    binds.require("all_bound", bound(binds, VAR));
    binds.require("no_redefinition", distinct_defs(binds, VAR));
    Bool::from_bool(true)
}
```

### `Given`

```rust
pub trait Given {
    fn given(self, guard: impl IntoIterator<Item = Bool>) -> Bool;
}
impl Given for Bool { .. } // self ∧ guard1 ∧ guard2 ∧ ..
```

「前提が成り立つときだけ問う」ための書き方です。ある違反が別の違反を必ず伴うとき、`given` で片方を除外すると、両方の種類が独立に生成されます。

```rust
exactly(1) { for i in binds.refs(VAR) { any {
    binds.mark(i, Bug::Killed, killed(binds, i)),
    binds.mark(i, Bug::SelfRef, constraint!(all {
        !visible(binds, i),
        defined_in_same_step(binds, i),
    })),
    binds.mark(i, Bug::Undefined, (!visible(binds, i)).given([
        !killed(binds, i),
        !defined_in_same_step(binds, i),
    ])),
} } }
```

### 統計(スレッドごと)

| シグネチャ | 内容 |
| --- | --- |
| `label_stats() -> Vec<LabelStat>` | このスレッドで生成したケースの、ラベルごとの出現回数 |
| `take_label_stats() -> Vec<LabelStat>` | 同上を取り出してリセット |
| `take_rejections() -> Vec<(Vec<String>, usize)>` | 棄却の理由と回数を取り出してリセット。理由は unsat core に出た `require` の名前(ソート済み)。名前のない制約だけで矛盾したときは空の組。unsat 以外の失敗は `Failure` の表示文字列。数えるのは `fill` の生成時だけ |

```rust
pub struct LabelStat {
    pub label: String,   // ラベルの名前
    pub declared: usize, // そのラベルを記録したケース数
    pub occurred: usize, // そのラベルが1回以上真になったケース数
}
```

- 数えるのは新しく生成したケースだけで、縮小の途中は数えません。
- テストのスレッドが終わるとき、一度も真にならなかったラベルが標準エラーに警告されます。テストが通っても表示されます。

```text
proptest-z3 warning: label `Delete` never occurred in 4 cases (test `each_kind_is_reported`); is it implied by another violation? consider `given`
```

- 起きないように制約した種類まで `mark` すると、この警告が出ます。狙った種類だけを `mark` してください。

## A.9 proptest に渡す

### `FillExt`

`Strategy<Value = Skeleton>` を実装するすべての型に実装されています。

| シグネチャ | 内容 |
| --- | --- |
| `fn fill<R>(self, names: NameSpec, rule: R) -> Filled<Self, R>` | `fill_with(Fill::Greedy, ..)` |
| `fn fill_with<R>(self, fill: Fill, names: NameSpec, rule: R) -> Filled<Self, R>` | 解き方を選ぶ |

`R: Fn(&Binds) -> Bool + 'static`。外の値を使うときは `move` クロージャにします。

`Filled<S, R>` は `Strategy<Value = Solved>` です。

### `Fill`

| 値 | 内容 |
| --- | --- |
| `Fill::Greedy`(既定) | 各穴のヒントを仮定として解く。ぶつかったら unsat core の中で評価順が最後のヒントを捨てて解き直す。`QF_LIA` のソルバを使う。決定的で速い |
| `Fill::Optimal` | 守れるヒントの数を最大化する(`Optimize`)。忠実だが、穴が多いと遅い |

`Greedy` では評価順で後ろの穴から希望を諦めるので、名前は先に評価される穴ほどヒントに近くなります。

### 生成と縮小の振る舞い

- 生成: 骨格と候補を作り、解けなければ(unsat)作り直します。作り直し1回が proptest の local reject 1回で、上限(`max_local_rejects`)を超えるとテストが中断します。
- 縮小: 縮めた入力が解けないとき、「その方向では失敗を再現できない」として戻り、別の方向を探します。
- 解いた結果(テキストと印・ラベル)が前にテストしたものと同じならテストを呼びません。骨格と候補が同じなら解き直しません。
- 縮小は不動点まで回しません。反例に無関係な行が残ることがあります。

## A.10 結果: `Solved`

`Clone`。z3 の型を含みません。

| シグネチャ | 内容 |
| --- | --- |
| `pub text: String` | 穴を埋めたテキスト |
| `fn holes(&self) -> impl Iterator<Item = (Hole, &FilledHole)>` | テキスト順のすべての穴 |
| `fn hole(&self, hole: Hole) -> &FilledHole` | |
| `fn span(&self, hole: Hole) -> Range<usize>` | `text` の中の位置(バイト) |
| `fn name(&self, hole: Hole) -> &str` | 入った名前 |
| `fn events(&self) -> &[Hole]` | 評価順に並べた穴。`events()[i]` が評価順 `i` の穴 |
| `fn target(&self, hole: Hole) -> Option<Hole>` | 参照が指す定義の穴。`Kill` なら削除した定義。定義の穴と、見えない参照は `None` |
| `fn marks<L: Label>(&self) -> Vec<(Hole, L)>` | 真になった `mark` |
| `fn labels<L: Label>(&self) -> Vec<L>` | 真になった `observe` |
| `fn has<L: Label>(&self, kind: L) -> bool` | `observe` の `kind` が真になったか |
| `fn numbered(&self) -> String` | 行番号(0始まり)付きのテキスト |

`target` の規則(A.2 と同じ):

- 候補は `reaching_defs`(字句的)のうち、同名で、間に同名の `Kill`(スコープを問わない)がないもの。
- その中で評価順が最後のものを返します。そのため、シャドーイングでは内側の定義、同じスコープでの再代入では最後の代入を指します。
- `Kill` が実際に消すのは同名の定義すべてです。`target` が返すのはそのうち1つ(上の規則で選んだもの)だけです。
- 制約の中で同じことを書くには `resolves_to(binds, i, k)` を使います(A.7)。境界は `target` の結果を変えません。

### `FilledHole`

```rust
pub struct FilledHole {
    pub space: Space,
    pub role: Role,
    pub name: String,
    pub span: Range<usize>, // text の中の位置(バイト)
}
```

### `Hole`

`pub struct Hole(pub usize)`。テキスト順の番号です。`Copy`, `Eq`, `Ord`, `Hash`。

### `Debug` の表示

反例の表示に使われます。テキスト(行番号付き)、真になった印、真になった `observe` のラベルを出します。

```text
Solved{
  0| let b
  1| let a = a
 marks: Undefined @ 14..15 `a`
 labels: Reassigned
}
```

印は「ラベル名 @ バイト範囲 名前(バッククォート囲み)」の形です。`marks:` と `labels:` の行は、空なら出ません。

## A.11 直接解く: `Skeleton::solve`

```rust
fn solve(&self, names: &Names, fill: Fill, rule: impl Fn(&Binds) -> Bool) -> Result<Solved, Failure>
```

proptest を通さずに1つの骨格を解きます。単体テストや、生成した入力を確かめるときに使います。

```rust
let skeleton = Skeleton::steps([frag!["def ", Frag::hole(VAR, Role::Def, Hint(0))]]);
let names = Names::default().with(VAR, ["a", "b"]);
let solved = skeleton.solve(&names, Fill::Greedy, |binds| bound(binds, VAR))?;
```

- 解けなければ `Err(Failure::Unsat { core })`(`core` は `require` の名前)、判定不能なら `Err(Failure::Unknown { reason })`。
- 骨格に穴がある名前空間の候補が空だと panic します(`no names for space ..`)。
- ラベルの統計は数えません(数えるのは `fill` の生成時だけ)。

## A.12 z3-toolkit から使うもの

規則の中では z3-toolkit の `constraint!` を使います(prelude から入ります)。proptest-z3 の例やテストで使っている形は次のとおりです。正確な構文は z3-toolkit のドキュメントを見てください。

| 形 | 意味 |
| --- | --- |
| `constraint!(e1, e2, ..)` | すべて成り立つ(連言) |
| `constraint!(x == y)` / `constraint!(x in 0..n)` | 比較・範囲 |
| `all { .. }` / `any { .. }` | 連言・選言 |
| `exactly(n) { .. }` / `at_most(n) { .. }` | ちょうど n 個・高々 n 個が真 |
| `{ for i in iter { e1, e2 } }` | 繰り返しで項を並べる。本体に複数の項を書くと、すべてが数え上げの対象になる |
| `if cond { e } else { e }`(Rust の `bool`) | 生成時に決まる条件で項を選ぶ |
| `e1 \|\| e2`,`!e` | 選言・否定 |

z3 の `Bool` は直接も組めます: `bound(binds, VAR) & bound(binds, FUNC)`、`Bool::from_bool(true)`(制約なし)、`Bool::or(&[..])`(空なら偽)、`Bool::and(&[..])`(空なら真)。

## A.13 棄却・panic になる条件

| 状況 | 結果 |
| --- | --- |
| 規則が unsat(例: 参照が1つもない骨格に `exactly(1) { for i in binds.refs(..) .. }`) | 生成時は local reject。縮小時は「通った」扱い |
| local reject が `max_local_rejects` を超える | テストが中断 |
| 骨格の穴がある名前空間を `NameSpec` に登録していない | panic(`no names for space ..`) |
| 同じ `Space` を `NameSpec` に2回登録 | panic |
| `space_with` の候補に空文字列か重複 | panic |
| `NameSpec::space` の正規表現が不正 | panic |

棄却が多いときは、`require` で制約に名前を付けて `take_rejections()` を見ます。形(strategy)が条件を満たし得ないことが多いので、条件を満たせる形を strategy 側で用意します(例: 削除後使用のテストでは、`del` の後に使用を置く形を確実に作る)。

- `proptest!` のテストは棄却が上限を超えると panic し、その後の `take_rejections()` に進めません。`TestRunner` で直接 `new_tree` を呼んで生成し、集計します
- 名前が1つだけの組は、形が原因のことがほとんどです(他の制約を外しても棄却される)。規則そのものの矛盾は、どの形でも満たせない組として現れます。最も多い組が原因とは限りません
- Z3 が返す組は最小とは限りません。疑わしい制約を1つずつ外して確かめます

```rust
use proptest::test_runner::TestRunner;
use proptest_z3::take_rejections;

#[test]
fn why_rejected() {
    let strategy = program().fill(names(), rule);
    let mut runner = TestRunner::default();
    for _ in 0..50 {
        if strategy.new_tree(&mut runner).is_err() {
            break; // 棄却が上限を超えた
        }
    }
    for (core, count) in take_rejections() {
        eprintln!("{count:>5} {core:?}");
    }
}
```

## A.14 テストを書くときの確認事項

LLM がテストを書くときに間違えやすい点です。

- 制約の中の番号は評価順、`Solved` の `Hole` はテキスト順。制約の中で `Hole` は作れない。
- 同じ文の右辺は左辺の定義を見ない。文の `Def` は、その文に含まれるスコープからも見えない(既定では再帰できない)。
- `Kill` はスコープを跨いで同名をすべて消す。
- `distinct_defs` はスコープを跨いで数える。`!distinct_defs` は「どこかに同名の定義がある」で、同じスコープでの再代入とは限らない。
- `Frag::boundary` は見える範囲を変えない。外側の定義はクロージャの中から普通に見える。キャプチャかどうかは `captured` / `boundaries_between` で判定する。
- `boundaries_between` は普通のスコープ(`Frag::scope`)を数えない。ブロックの中から外側の定義を指しても、キャプチャにはならない。
- 境界の中身は書かれた位置で評価されたものとして扱う。クロージャ本体の `Kill` は、その後の参照にも効く。
- 「未定義」を他の違反と区別するには `given` で前提を足す。そうしないと、ある種類が一度も生成されないことがある(警告で気づける)。
- `mark` の戻り値を制約に入れ忘れると、ただの記録になる。
- 異常系の `exactly(1)` は、対象の穴が骨格に1つもないと unsat。形の側で対象の穴を確実に作る。
- 正常系の検査器(テストの期待値)は、`rules` と独立に書く。同じ意味論で書くと同じ誤りを共有する。
- 規則のクロージャは `'static`。外の値は `move` で取り込む。
- `require` は値を返さないので `constraint!` の中に書かない。規則の本体で呼び、同じ制約を戻り値に重ねて入れない。
