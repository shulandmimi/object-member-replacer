
const obj = {};

obj.namespace = 10;
obj.namespace = 10;
obj.namespace = 10;
obj['namespace' + { 'namespace': 2 }['namespace'] ] = 10;
obj['namespace' + 1] = 10;

const obj1 = {
    namespace: 20,
};
const obj2 = {
    namespace: 20,
};
const obj3 = {
    ["namespace"]: 20,
};
const obj4 = {
    namespace: {
        namespace: {
            namespace: {},
        },
    },
    ["namespace"]: {},
    ["a" + "namespace"]: {},
    ["a" + obj4["namespace"]]: {}
};

const { namespace } = obj4;
const obj5 = Object.assign({}, {
    namespace: 2,
})

const obj6 = {
    namespace() {
        return 10;
    }
}

const { namespace: n2 } = obj4;
const { ['namespace']: n3 } = obj4;

console.log(n2);

asserts.equal(obj1, obj2, obj3, obj4, obj5, obj6, n2, n3);