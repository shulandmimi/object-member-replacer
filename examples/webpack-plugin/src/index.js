const fooFn = require('./foo');

var a = {};

fooFn();

a.foo_foo_bar_foo_foo = { f1: 0 };
console.log(a.foo_foo_bar_foo_foo);
a.foo_foo_bar_foo_foo = { f2: 3 };
console.log(a.foo_foo_bar_foo_foo);
a.foo_foo_bar_foo_foo = { f3: 0 };
console.log(a.foo_foo_bar_foo_foo);
function nested1() {
    a.foo = Math.random();
    console.log(new Error());
}
function foo() {
    a.foo_foo_bar_foo_foo = { f4: 4 };
    console.log(a.foo_foo_bar_foo_foo);
    a.foo_foo_bar_foo_foo = { f5: 5 };
    nested1();
    console.log(new Error());
}

console.log(a.foo_foo_bar_foo_foo);
a.foo_foo_bar_foo_foo = { f6: 6 };

console.log(a.foo_foo_bar_foo_foo);

foo();

const { foo_foo_bar_foo_foo: bar } = a;

console.log(bar);
