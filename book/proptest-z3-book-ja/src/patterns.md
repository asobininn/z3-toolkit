# よくあるパターン

この章では、名前解決のテストでよく使う制約の書き方を紹介します。制約を作る参考にご活用ください。

## 名前解決の境界条件

ランダムに作った正常系では、名前解決器が間違えやすい状況はめったに出てきません。`bound` で正常系にしたうえで、そうした状況を `any` で必ず含めます。

### 外に同じ名前がある

```rust,ignore
let x
let x = x       // 右辺は1行目の x を指す
```

誤って左辺の `x` を先に登録してしまう名前解決器は、右辺の自分自身に結び付けてしまいます。

```rust,ignore
program().fill(names(), |binds| constraint!(
    bound(binds, VAR),
    any { for i in binds.uses(VAR) { defined_in_same_step(binds, i) } },
))
```

`bound` の下では、すべての参照が見えている、そのうえで同じステップに同じ名前の定義があるので、右辺は必ず外側の定義を指します。

### シャドーイング

```rust,ignore
let x
{
    let x
    print x     // 内側の x を指す
}
```

外側の `x` も名前は一致するので、スコープを正しく辿らない名前解決器は外側を選んでしまいます。

```rust,ignore
/// 参照 `i` が指す定義が、外側のスコープにある同じ名前の定義を隠している
fn hides_outer(binds: &Binds, i: usize) -> Bool {
    let cases: Vec<Bool> = reaching_defs(binds, i)
        // i が見える可能性のある定義の列挙
        .flat_map(|k| reaching_defs(binds, i).map(move |q| (k, q))) 
        // q が k より外側にある(同じスコープは含まない)
        .filter(|&(k, q)| !binds.encloses(k, q)) 
        // i が実際に k を指す、同じ名前のもの
        .map(|(k, q)| constraint!(resolves_to(binds, i, k), binds.name[q] == binds.name[k]))
        .collect();
    Bool::or(&cases)
}

program().fill(names(), |binds| constraint!(
    bound(binds, VAR),
    any { for i in binds.uses(VAR) { hides_outer(binds, i) } },
))
```

### テスト

どちらの例も、名前解決の結果を `Solved::target` と比べます。

```rust,ignore
let resolved = resolve(&s.text);    // 参照の位置 → 定義の位置
for (hole, filled) in s.holes().filter(|(_, h)| h.role.is_ref()) {
    let expected = s.target(hole).map(|def| s.span(def).start);
    prop_assert_eq!(resolved.get(&filled.span.start).copied(), expected, "{:?}", s);
}
```

定義へのジャンプや参照検索のような「どこを指すか」を返す機能も、同じ形でテストが可能です。

## 巻き上げ

Rust の関数宣言のように、後ろで定義した名前も、同じスコープの前の位置から見える言語があります。これを「巻き上げ」と呼びます。

```rust,ignore
call f      // まだ定義されていないが、呼べる
fn f
```

`visible` や `bound` は「評価順で前の定義だけが見える」という規則で書かれています。そのため巻き上げのある名前空間では、可視性の規則を自分で書く必要があります。

```rust,ignore
/// 参照 `i` から、前後に関係なく、囲むスコープにある同じ名前の定義が見える
fn hoisted(binds: &Binds, i: usize) -> Bool {
    let defs: Vec<Bool> = (0..binds.len())
        .filter(|&k| binds.role[k] == Role::Def && binds.space[k] == binds.space[i])
        .filter(|&k| binds.encloses(k, i))
        .map(|k| constraint!(binds.name[k] == binds.name[i]))
        .collect();
    Bool::or(&defs)
}
```

正常系では、すべての呼び出しが巻き上げで見えていることを要求します。そのうえで、巻き上げがなければ見えていない呼び出し(前方参照)を必ず含めます。

```rust,ignore
program().fill(names(), |binds| constraint!(
    bound(binds, VAR),
    all { for i in binds.refs(FUNC) { hoisted(binds, i) } },
    // 前方参照が少なくとも1つある
    any { for i in binds.refs(FUNC) { hoisted(binds, i) & !visible(binds, i) } },
))
```

ここでは、既存の `visible` を「順序ありの可視性」として使用しています。`hoisted` が真で `visible` が偽なら、巻き上げによって初めて見える、ということです。

## 再帰と `let rec`

ここでは、同じ言語に次の両方があり、処理系がそれぞれを正しく扱うかを確かめます。

```rust,ignore
fn f {
    call f      // 非再帰の関数なら、ここから f は見えない
}
fn rec g {
    call g      // 再帰の関数なら見える
}
```

再帰かどうかは、制約ではなくステップの切り方で決まります。

- ステップの中では、`Def` が評価順の最後に回る
- 名前と本体を1つのステップにすると、本体から名前は見えない(非再帰)
- 名前だけのステップを本体より前に置くと、本体から見える(再帰)

```rust,ignore
fn stmt() -> impl Strategy<Value = Vec<Frag>> {
    let simple = prop_oneof![
        VAR.def().prop_map(|x| vec![frag!["let ", x, "\n"]]),
        VAR.refer().prop_map(|y| vec![frag!["print ", y, "\n"]]),
        FUNC.refer().prop_map(|f| vec![frag!["call ", f, "\n"]]),
    ];
    simple.prop_recursive(3, 16, 4, |inner| {
        let body = prop::collection::vec(inner, 1..4).prop_map(|stmts| stmts.concat());
        (any::<bool>(), FUNC.def(), body).prop_map(|(rec, f, body)| {
            let body = frag![" {\n", Frag::scope(body), "}\n"];
            if rec {
                vec![frag!["fn rec ", f], body]   // 名前のステップ → 本体のステップ
            } else {
                vec![frag!["fn ", f, body]]       // 1つのステップ
            }
        })
    })
}

fn program() -> impl Strategy<Value = Skeleton> {
    prop::collection::vec(stmt(), 1..8).prop_map(|stmts| Skeleton::steps(stmts.concat()))
}
```

テストは「名前解決の境界条件」と同じく `Solved::target` と比べます。

### 注意点

```rust,ignore
fn rec f { call g } and g { call f }
```

のような相互再帰はこの方法では作れません。`f` の本体はテキスト上の `g` の定義より前にあるのに、`g` を見る必要があるからです。これは「グループの中だけの巻き上げ」なので、前の節の `hoisted` の候補を、そのグループの定義に絞って書きます。

## クロージャ

```rust,ignore
let x
let g = || {
    let h = || {
        print x     // 2段のクロージャを越えて x をキャプチャする
    }
}
```

クロージャの外の名前を参照すると、処理系はその名前をクロージャの中へ持ち込む必要があります(キャプチャ)。ここではキャプチャされる参照を処理系が正しく見つけるかを確かめます。

### 骨格

ブロックとクロージャは可視性の規則が全く同じです。違うのは「ここで実行時のフレームが変わる」という点だけです。この違いを表すために、クロージャ本体は `Frag::scope` の代わりに `Frag::boundary` で作ります。

```rust,ignore
fn closure(f: Frag, stmts: Vec<Frag>) -> Frag {
    frag!["let ", f, " = || {\n", Frag::boundary(stmts.into_iter().map(|s| frag![s, "\n"])), "}"]
}
```

2段の入れ子の中の参照は、ランダムな形ではめったに生まれません。専用の骨格を用意します。

```rust,ignore
fn program_with_nested_closures() -> impl Strategy<Value = Skeleton> {
    let head = prop::collection::vec(VAR.def().prop_map(|x| frag!["let ", x]), 1..3);
    let deep = (VAR.def(), VAR.def(), VAR.refer()).prop_map(|(g, h, u)| {
        closure(g, vec![closure(h, vec![frag!["print ", u]])])
    });
    (head, prop::collection::vec(stmt(), 0..5), deep).prop_map(|(head, body, deep)| {
        Skeleton::steps(head.into_iter().chain(body).chain([deep]).map(|s| frag![s, "\n"]))
    })
}
```

### 制約

`rules::captured(binds, i)` は、参照 `i` が境界の外の定義を指すことを表します。何段の境界を越えるかは `binds.boundaries_between(k, i)` で数えられるので、2段以上越えるキャプチャも自分で書けます。

```rust,ignore
/// 参照 i が、2段以上のクロージャを越えて定義を指す
fn relayed(binds: &Binds, i: usize) -> Bool {
    let cases: Vec<Bool> = reaching_defs(binds, i)
        .filter(|&k| binds.boundaries_between(k, i) >= 2)
        .map(|k| resolves_to(binds, i, k))
        .collect();
    Bool::or(&cases)
}
```

ここで名前の一致ではなく `resolves_to` を使用しているのは、シャドーイングのためです。

```rust,ignore
let x
let g = || {
    let x
    print x    // 内側の x を指す。キャプチャではない
}
```

### テスト

すべての参照に、キャプチャかどうかの印を付けます。`mark` の戻り値は捨てているので、制約にはなりません。

```rust,ignore
#[derive(Debug, Clone, Copy, PartialEq)]
struct Captured;

proptest! {
    #[test]
    fn captures_are_found(s in program_with_nested_closures().fill(names(), |binds| {
        for i in binds.refs(VAR) {
            binds.mark(i, Captured, captured(binds, i));
        }
        constraint!(
            bound(binds, VAR),
            any { for i in binds.refs(VAR) { relayed(binds, i) } },
        )
    })) {
        let expected: HashSet<usize> =
            s.marks::<Captured>().into_iter().map(|(hole, _)| s.span(hole).start).collect();
        prop_assert_eq!(captures(&s.text), expected, "{:?}", s);
    }
}
```

### 注意点

- `captured` は、トップレベルの定義への参照もキャプチャに数えます。グローバル変数をキャプチャと呼ばない言語では、期待値を作るときに除いてください。
- `captured` は順序ありの可視性に従います。巻き上げる名前空間では使えません。
