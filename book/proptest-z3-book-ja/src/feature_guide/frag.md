# Frag と Skeleton

穴と文字列を並べて入力の構造を作ります。組み立て中のものを `Frag`(断片)、完成したものを `Skeleton` と呼びます。

- `Frag`: 文字列と穴の並び。`frag!` や `Frag::concat` でつなげることができる
- `Skeleton`: `Skeleton::steps`で、`Frag` の列を**ステップ**に区切ったもの

## `frag!` でつなげる

`frag!` は渡したものを順につなげて1つの `Frag` にします。

```rust
(VAR.def(), VAR.refer()).prop_map(|(x, y)| frag!("let ", x, " = ", y))
```

`frag!` に渡せる値は次のとおりです。

| 渡せるもの | 例 |
| --- | --- |
| 文字列 | `"let "` |
| 数値 | `42` |
| `Frag`(穴を含む) | `x`、`frag![..]` |
| `Vec<Frag>` | 要素を順につなげる |
| `Option<Frag>` | `None` なら何も足さない |
| タプル(4要素まで) | 要素を順につなげる |
| `Text(x)` | `Display` を実装したユーザ型 |

ユーザの型は `Text` で包むと `Display` の結果が文字列として入ります。

```rust
#[derive(Debug, Clone, Copy)]
enum Op { Add, Sub, }

impl Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self { Op::Add => "+", Op::Sub => "-", })
    }
}

(VAR.refer(), prop_oneof![Just(Op::Add), Just(Op::Sub)], VAR.refer())
    .prop_map(|(lhs, op, rhs)| frag!(lhs, " ", Text(op), " ", rhs))
```

## 手で組む

`frag!` を使わずに、関数で組み立てることもできます。

| 関数 | 内容 |
| --- | --- |
| `Frag::text(s)` | 文字列だけの `Frag` |
| `Frag::hole(..)` | 穴1つの `Frag` |
| `Frag::concat(frags)` | 順につなげる |
| `Frag::join(sep, frags)` | 間に区切りを挟んでつなげる |

`Frag::join` は、引数の並びのような区切りのある列に使います。

```rust
prop::collection::vec(VAR.refer(), 0..4)
    .prop_map(|args| frag!["print(", Frag::join(", ", args), ")"])
```

## ステップ

`Skeleton::steps` は、渡した列の要素を1つずつ**ステップ**として区切ります。ステップは、定義した名前が見えるようになる単位です。言語の「文」に当たる単位で区切ってください。

```rust
fn program() -> impl Strategy<Value = Skeleton> {
    prop::collection::vec(stmt(), 1..8)
        .prop_map(|stmts| Skeleton::steps(stmts.into_iter().map(|s| frag![s, "\n"])))
}
```

### 評価順

穴は、テキストの順ではなく**評価順**に並べて扱われます。評価順は次のように決まります。

- ステップは、並んでいる順に評価される
- ステップの中では、`Use` の穴が先、`Def` と `Kill` の穴が後に評価される

そのため、`let x = y` の `y` は `x` より先に評価されます。`let a = a` の右辺の `a` は、同じ文の左辺の `a` を指しません。Rust の `let` と同じ振る舞いです。

制約は、この評価順をもとに書きます。詳しくは [Binds と規則](./binds.md) で説明します。

## スコープ

`Frag::scope` は、入れ子のスコープを作ります。渡した列の要素は `Skeleton::steps` と同じようにステップとして区切られ、中で定義した名前はスコープの外から見えません。

ブロック `{ .. }` を足すと、次のようになります。

```rust
fn stmt() -> impl Strategy<Value = Frag> {
    let simple = prop_oneof![
        VAR.def().prop_map(|x| frag!["let ", x]),
        (VAR.def(), VAR.refer()).prop_map(|(x, y)| frag!["let ", x, " = ", y]),
        VAR.refer().prop_map(|y| frag!["print ", y]),
    ];
    simple.prop_recursive(3, 16, 4, |inner| {
        prop::collection::vec(inner, 1..4).prop_map(|stmts| {
            frag!["{\n", Frag::scope(stmts.into_iter().map(|s| frag![s, "\n"])), "}"]
        })
    })
}
```

外側から見ると、ブロック全体が1つのステップです。ブロックの中の文は、それぞれがスコープの中のステップになります。

- ブロックの中からは、ブロックより前に外で定義された名前が見える
- ブロックの中で定義した名前は、ブロックの後では見えない
- ブロックの中で、外と同じ名前を定義し直せる(シャドーイング)

スコープは、ブロックのような字句的な入れ子だけを扱います。

> **`Kill` とスコープ**: `Kill` はスコープに関係なく、その名前を以降すべて見えなくします。外で定義した名前をブロックの中で `del` すると、ブロックを出た後も見えません。
