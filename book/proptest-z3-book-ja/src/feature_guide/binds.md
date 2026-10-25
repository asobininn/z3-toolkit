# Binds と規則

名前どうしの関係は**規則**として書きます。規則は `&Binds` を受け取って Z3 の `Bool` を返す関数で、`fill` に渡します。

```rust
program().fill(names(), |binds| bound(binds, VAR))
```

この節では、規則の材料になる `Binds` と、よく使う関係を表す述語を説明します。

## Bind: 穴の番号

`Binds` は、骨格の穴を[評価順](./frag.md#評価順)に並べたものです。規則の中では、穴を評価順の番号 `i` で指します。番号が小さいほど先に評価されるので、「`i` より前の穴」は番号の大小で表せます。

穴の番号は名前空間と役割で取り出します。どれも評価順に並んだ番号を返します。

| 書き方 | 取り出す穴 |
| --- | --- |
| `binds.defs(S)` | 名前空間 S の `Def` |
| `binds.uses(S)` | 名前空間 S の `Use` |
| `binds.kills(S)` | 名前空間 S の `Kill` |
| `binds.refs(S)` | 名前空間 S の参照(`Use` と `Kill`) |
| `binds.of(S, role)` | 名前空間 S の、役割 `role` の穴 |

`binds.name[i]` は穴 `i` に入る名前を表す Z3 の `Int` です。値は候補の番号なので、`binds.name[p] == binds.name[q]` で「穴 p と穴 q に同じ名前が入る」と書けます。

```rust
// VAR の定義の名前は、すべて異なる
fn no_redefinition(binds: &Binds) -> Bool {
    constraint!(
        distinct(binds.defs(VAR).map(|i| &binds.name[i])),
    )
}
```

### 生成時に決まる関係

ステップやスコープは構造を作った時点で決まっています。このような関係は Z3 の `Bool` ではなく、 Rust の値 (`bool` や `usize`) で返されます。

| 書き方 | 意味 |
| --- | --- |
| `binds.same_step(p, q)` | 穴 p と q が同じステップにある |
| `binds.encloses(outer, inner)` | 穴 outer で定義した名前が、穴 inner から見える範囲にある |
| `binds.boundaries_between(outer, inner)` | 穴 inner から穴 outer の定義まで、越えるクロージャの数([クロージャ](../patterns.md#クロージャ)を参照) |

Rust の `bool` なので、`constraint!` の中では `if` で制約を絞るのに使います。

```rust
// 同じステップの中では、参照と定義に違う名前を使う
constraint!(for p in binds.uses(VAR) {
    for q in binds.defs(VAR) {
        if binds.same_step(p, q) {
            binds.name[p] != binds.name[q]
        }
    }
})
```

## 束縛の述語

名前解決でよく使用する関係は関数として用意されています。どれも Z3 の `Bool` を返すので、`constraint!` の中でそのまま組み合わせられます。

| 書き方 | 意味 |
| --- | --- |
| `visible(binds, i)` | 参照 `i` が、見えていて削除されていない定義を指す |
| `resolves_to(binds, i, k)` | 参照 `i` が、定義 `k` を指す(同名の定義のうち評価順で最後のもので、間に削除がない) |
| `killed(binds, i)` | 参照 `i` が、削除済みの名前を指す |
| `bound(binds, S)` | 名前空間 S の参照が、すべて見えている |
| `unbound(binds, S, n)` | 名前空間 S の、見えていない参照がちょうど n 個 |
| `defined_in_same_step(binds, i)` | 参照 `i` と同じステップに、同じ名前の定義がある |
| `distinct_defs(binds, S)` | 名前空間 S の定義の名前が、すべて異なる |

「見えている」は、次をすべて満たすことです。

- その名前の定義が、参照より前に評価される
- その定義のスコープが、参照を囲む
- 定義から参照までの間に、その名前が削除されていない

上の規則で足りないときは、次の2つを使って自分で制約を作れます。`visible` や `resolves_to` も、この2つの組み合わせで書かれています。

| 書き方 | 返すもの |
| --- | --- |
| `reaching_defs(binds, i)` | 参照 `i` の候補の定義。同じ名前空間で、`i` より前にあり、`i` を囲むスコープにあるもの |
| `kills_between(binds, k, i)` | `k` と `i` の間で、`i` と同じ名前空間の名前を削除している穴 |

## 参照の3つの状態

参照は、次の3つのどれかの状態にあります。

| 状態 | 書き方 |
| --- | --- |
| 見えている | `visible(binds, i)` |
| 削除済み | `killed(binds, i)` |
| 未定義 | `(!visible(binds, i)).given([!killed(binds, i)])` |

`!visible` は「削除済み」と「未定義」の両方を含みます。Getting Started の異常系を `del` を含む言語でそのまま使うと、削除済みの名前を使う入力も「未定義」として生成されます。

`given` は前提が成り立つときだけ条件を問う書き方です。詳細は[ラベル](./labels.md)で説明します。
