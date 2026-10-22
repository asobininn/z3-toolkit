# proptest-z3

[![crates.io](https://img.shields.io/crates/v/proptest-z3.svg)](..)
[![docs.rs](https://docs.rs/proptest-z3/badge.svg)](..)
[![Book](https://img.shields.io/badge/book-English-blue.svg)](..)
[![Book](https://img.shields.io/badge/book-日本語-blue.svg)](..)
[![z3-toolkit](https://img.shields.io/badge/with-z3--toolkit-orange.svg)](https://crates.io/crates/z3-toolkit)

proptest-z3 は、要素間の依存関係を持つテスト入力を生成するための、[proptest](https://crates.io/crates/proptest) の拡張です。
入力の構造は通常どおり proptest の Strategy で定義し、要素どうしの関係は [Z3](https://crates.io/crates/z3) の制約で指定します。
制約を反転することで、指定した数の違反を含む入力を生成でき、違反した箇所も特定できます。縮小時にも要素間の関係は保たれます。

## Setup

```toml
[dev-dependencies]
proptest = "1"
proptest-z3 = "0.1"
```

Z3 のライブラリが必要です。インストール方法やリンク設定については、[z3クレートのREADME](https://github.com/prove-rs/z3.rs) を参照してください。

## Usage

### 依存関係のある入力を作る

例として、1行に1文の小さな言語を使います。

```text
let x        // x を定義する
let x = y    // y を使ってから x を定義する
print x      // x を使う
```

テストする対象は、この言語の名前解決器 `undefined_refs(src) -> Vec<usize>` です。未定義の名前を参照した位置(バイト)を返します。

名前を書く場所には、実際の名前の代わりに穴を置きます。`def`は定義、`refer`は参照です。どの穴にどの名前を入れるかは Z3 が決めるため、生成器の中で名前を追跡する必要はありません。

```rust
use proptest::prelude::*;
use proptest_z3::prelude::*;

const VAR: Space = Space::new("var");

fn program() -> impl Strategy<Value = Skeleton> {
    prop::collection::vec(stmt(), 1..8)
        .prop_map(|stmts| Skeleton::steps(stmts.into_iter().map(|s| frag![s, "\n"])))
}

fn stmt() -> impl Strategy<Value = Frag> {
    prop_oneof![
        VAR.def().prop_map(|x| frag!["let ", x]),
        (VAR.def(), VAR.refer()).prop_map(|(x, y)| frag!["let ", x, " = ", y]),
        VAR.refer().prop_map(|y| frag!["print ", y]),
    ]
}

// 名前の候補: [a-c] に合う、互いに異なる名前を3個
fn names() -> NameSpec {
    NameSpec::new().space(VAR, "[a-c]", 3)
}

proptest! {
    // 全ての参照が、見ている名前を指す
    #[test]
    fn defined_names_resolve(s in program().fill(names(), |binds| bound(binds, VAR))) {
        prop_assert_eq!(undefined_refs(&s.text), vec![], "{:?}", s);
    }
}
```

### 制約を変えて異常系の入力を生成する

変更するのは `fill` に渡す制約だけです。

```rust
// 正常系
program().fill(names(), |binds| bound(binds, VAR))

// 異常系: 参照できない名前を指す参照がちょうど1つある
program().fill(names(), |binds| constraint!(
    exactly(1) { for i in binds.refs(VAR) {
        binds.mark(i, Undefined, !visible(binds, i))
    } },
))
```

`mark` で印をつけた穴は、制約を解いた後に位置を特定できます。これにより、名前解決器が違反箇所を正しく報告するか検証できます。

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
struct Undefined;

proptest! {
    #[test]
    fn undefined_name_is_reported_where_it_is(s in program().fill(names(), |binds| constraint!(
        exactly(1) { for i in binds.refs(VAR) {
            binds.mark(i, Undefined, !visible(binds, i))
        } },
    ))) {
        let [(hole, Undefined)] = s.marks::<Undefined>()[..] else {panic!("{s:?}")};
        prop_assert_eq!(undefined_refs(&s.text), vec![s.span(hole).start], "{:?}", s);
    }
}
```

名前解決器に、`let x = y` で右辺を調べる前に左辺を定義してしまうバグがあると、次のように失敗します。

```text
running 2 tests
test undefined_name_is_reported_where_it_is ... FAILED
test defined_names_resolve ... ok

minimal failing input: s = Solved{
  0| let b
  1| let a = a
 marks: Undefined @ 14..15 `a`
}
```

正常系のテストが通っていても、本来エラーにすべき入力を受け入れてしまうバグは残り得ます。異常系の入力も生成することで、こうしたバグを検出できます。

### 複数の壊れ方を作り分ける

名前空間ごとに壊すかどうかを決め、違反を種類ごとに作り分けることもできます。下の例では、変数の再定義を許しつつ、参照のうちちょうど1つに「削除後の使用」「自己参照」「未定義参照」のいずれかの違反を持たせます。

```rust
program().fill(names(), |binds| {
    constraint!(
        // 同名の変数を定義しなおす
        !&distinct_defs(binds, VAR),
        // 関数の参照はすべて有効
        bound(binds, FUNC),
        // 変数の参照のうち、ちょうど1つが3通りのどれかで壊れる
        exactly(1) { for i in binds.refs(VAR) { any {
            binds.mark(i, Broken::Delete, killed(binds, i)),
            binds.mark(i, Broken::SelfRef, constraint!(all {
                !visible(binds, i),
                defined_in_same_step(binds, i),
            })),
            binds.mark(i, Broken::Undefined, (!visible(binds, i)).given([
                !killed(binds, i),
                !defined_in_same_step(binds, i),
            ])),
        } } },
    )
})
```

違反の条件どうしが重なっていたり、ある違反が別の違反を必ず伴ったりすると、一部の種類が意図どおりに生成されないことがあります。上の例で `given` を外した場合などです。このとき、テストが成功していても、一度も現れなかったラベルが警告されます。

```text
proptest-z3 warning: label `Delete` never occurred in 4 cases (test `each_kind_is_reported`); is it implied by another violation? consider `given`
```

`given` で前提を明示すると、違反どうしを独立にできます。

ほかにも、次のことができます。詳しくは [Book](..) を見てください。

- ブロックによる入れ子のスコープ
- 各参照がどの定義を指すかの取得
- 生成が棄却ばかりになるときの原因の集計

## Scope

proptest-z3 は、定義と参照を含む小さなテスト入力のためのクレートです。本クレートで使用される「名前」とは、変数名やテーブル名、行のキーのように、作られてから別の場所で指されるものを広く指します。

- 扱える関係は、名前どうしの等しさだけです。型や値の条件は `constraint!` で足せますが、整数などの値そのものを Z3 に選ばせる機能はまだありません。
- スコープは、ブロックのような字句的な入れ子だけを扱います。
- 入力の構造は proptest が決めるため、構造が制約を満たせないと生成が棄却されます。棄却が多いときは、条件を満たせる構造を Strategy の側で用意してください。
- 穴が多いほど、制約を解くのに時間がかかります。
