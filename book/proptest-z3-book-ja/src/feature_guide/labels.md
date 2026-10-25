# ラベル

規則の中で生成した入力に**ラベル**を付けられます。ラベルは次の3つの用途があります。

- どの穴を、どの種類の誤りとして壊したかを記録する(`mark`)
- 制約にはせず、生成された入力を分類する(`observe`)
- 制約に名前をつけ、棄却の原因を調べる(`require`)

ラベルには `Copy + Debug + PartialEq` を実装した任意の型を使用できます。

```rust
/// 規則で壊した種類
#[derive(Debug, Clone, Copy, PartialEq)]
enum Bug {
    Unbound, // 未定義の名前を使う
    Killed,  // 削除済みの名前を使う
}

/// 生成された入力の分類
#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    Redefined, // 同じ名前の定義が2回以上ある
}
```

## mark: 壊した場所を記録する

`binds.mark(i, kind, cond)`は、`cond` をそのまま返し、同時に「`cond` が成り立つなら、穴 `i` は `kind` として壊れている」を記録します。

```rust
|binds| constraint!(
    exactly(1) { for i in binds.refs(VAR) {
        binds.mark(i, Bug::Unbound, !visible(binds, i))
    } },
)
```

解いた後は、 `s.marks::<Bug>()` で `(穴, ラベル)` の列を受け取ります。ここでは「壊れているのがちょうど1箇所」しか扱わないため、次のような取り出す関数を用意しておくと便利です。

```rust
fn the_mark(s: &Solved) -> (usize, Bug) {
    let [(hole, bug)] = s.marks::<Bug>()[..] else {
        panic!("one mark expected: {s:?}")
    };
    (s.span(hole).start, bug)
}
```

## given: 違反条件の重複を避ける

ある違反を起こすと、別の違反も必ず起きてしまうことがあります。例えば、関数の呼び出しに「未定義の関数を呼ぶ」と「引数の数が合わない」の2種類の違反を用意したとします。引数の数の違反を「見えている定義と数が合わない」と書くと、未定義の関数の呼び出しは、引数の数の違反も同時に満たします。

違反を種類ごとに数えると、未定義の関数を呼ぶ入力では違反が2つ数えられます。そのため `exactly(1)` で1か所だけ壊そうとすると、この入力は作れず、一度も生成されません。

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
enum CallBug {
    Unbound,
    ArityMismatch,
}

// 未定義の呼び出しは ArityMismatch も満たすので、Unbound が一度も出ない
|binds| constraint!(
    exactly(1) {
        for i in binds.uses(FUNC) { binds.mark(i, CallBug::Unbound, !visible(&binds, i)) },
        for i in binds.uses(FUNC) { binds.mark(i, CallBug::ArityMismatch, arity_mismatch(&binds, i)) },
    },
)
```

引数の数の違反を、`given` で「関数が見えているとき」に限って問うと、2つの違反を1つずつ独立に作れます。

```rust
|binds| constraint!(
    exactly(1) {
        for i in binds.uses(FUNC) { binds.mark(i, CallBug::Unbound, !visible(&binds, i)) },
        for i in binds.uses(FUNC) {
            binds.mark(i, CallBug::ArityMismatch, arity_mismatch(binds, i).given([visible(&binds, i)]))
        },
    },
)
```

`cond.given([前提..])` は、`cond` とすべての前提の論理積を表します。したがって、前提が偽なら式全体も偽です。前提が成り立つ場合に `cond` も成り立つことを要求したいときに使用します。

## 一度も出ないラベルの警告

一度も真にならなかったラベルは、テストの終わりに警告されます。テストは通っていても、狙った入力が一度も生成されなかったことを知らせるためです。

原因は大きく2つあります。**作れない**か、**出にくい**かです。

作れない場合:

- **構造が足りない**: 削除の後に参照が来る構造がなければ、削除後の参照は作れません。Strategy 側で、その構造を用意します
- **違反が重複する**: ある違反が別の違反も満たすため、`exactly(1)` では片方だけの入力を作れません。`given` で条件を分けます
- **他の制約で禁じている**: `bound` などで起こらないようにした種類まで `mark` すると、そのラベルは付きません
- **名前候補が偏っている**: 候補が少ないと未定義参照を作れず、多すぎると再定義などの衝突が起きにくくなります

出にくい場合:

- **条件が稀**: `observe` で記録する条件は分布を変えないため、生成されたケース数の範囲では一度も真にならないことがあります。構造の出やすさを調整するか、その種類だけを狙うテストを分けます

迷ったら、その種類だけを `exactly(1)` で狙うテストを書きます。すべて棄却されるなら作れません。`require` に名前を付け、`take_rejections` でどの制約に当たったかを調べます。

## observe: 分類する

`binds.observe(kind, cond)`は、`cond` を制約にはせず、生成された入力で `cond` が成り立ったかどうかだけを記録します。生成される入力の分布は変わりません。

`observe` は `Bool` を返さないので、`constraint!` の中には書けません。規則の本体で先に呼んでおき、制約は別に返します。

```rust
fn bound_observing_redefinition(binds: &Binds) -> Bool {
    binds.observe(Shape::Redefined, !distinct_defs(binds, VAR));
    bound(binds, VAR)
}
```

記録したラベルは、`s.labels::<L>()` や `s.has(kind)` で確認できます。次のテストは、`Shape::Redefined` が付いた入力に、実際に同じ名前の定義が2回以上あるかを確かめています。

```rust
proptest! {
    #[test]
    fn observe_classifies(s in program().fill(names(), bound_observing_redefinition)) {
        let defs: Vec<&str> = s.holes()
            .filter(|(_, h)| h.space == VAR && h.role == Role::Def)
            .map(|(_, h)| h.name.as_str())
            .collect();
        let unique: HashSet<_> = defs.iter().collect();
        prop_assert_eq!(s.has(Shape::Redefined), unique.len() < defs.len(), "{:?}", s);
    }
}
```

## 出現回数と未出現の警告

それぞれのラベルは、何回出現したかが数えられています。

```rust
eprintln!("{:?}", label_stats());
```

`label_stats()` はラベルごとの出現回数を返します。`take_label_stats()` は返し、カウントをリセットします。回数はスレッドごとに数えられます。

一度も現れなかったラベルは、テストのスレッドが終わるときに標準エラーへ警告されます。テストが通っていても表示されます。

```text
proptest-z3 warning: label `Killed` never occurred in 259 cases (test `tests::my_test`); is it implied by another violation? consider `given`
```

## require: 棄却の原因を調べる

構造が制約を満たせないと、生成は棄却されます。棄却が多いときは、`require` で原因を調べます。

`binds.require(label, cond)` は、`cond` に名前を付けて制約に加えます。`mark` と違って値を返さないので、`constraint!` の中には書かず、規則の本体で呼びます。

規則の戻り値は名前のない制約で、棄却の理由には出ません。名前のない制約と矛盾したときは、名前付きの側だけが理由に出ます。調べたい制約はすべて `require` に移し、戻り値は `Bool::from_bool(true)` にします。同じ制約を `require` しつつ戻り値にも入れると、そのラベルは理由に出なくなります。

次の規則では、自己参照を含み、すべての参照が見えていて、再定義のない入力を求めています。

```rust
fn rule(binds: &Binds) -> Bool {
    binds.require("self_ref", constraint!(any { for i in binds.uses(VAR) { defined_in_same_step(binds, i) } }));
    binds.require("all_bound", bound(binds, VAR));
    binds.require("no_redefinition", distinct_defs(binds, VAR));
    Bool::from_bool(true)
}
```

自己参照の右辺を見えるようにするには、同じ名前の定義が前に必要です。しかし、それは再定義にもなるため、3つの制約を同時には満たせません。このような矛盾は、`take_rejections()` に現れるラベルの組から見つけられます。

Z3 が返す組は最小とは限らず、原因と関係のない制約が含まれることもあります。疑わしい制約を1つずつ外し、棄却が解消するか確かめます。
