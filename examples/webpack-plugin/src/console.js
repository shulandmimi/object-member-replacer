const namespace = "webpack-plugin";

function foo(bar) {
    return (window.vvv = (bar + "xxaa"));
}
console.log(namespace + foo("xxxxx foo arg"));
console.log(namespace + foo("xxxxx foo arg"));
console.log(foo("xxxxx foo arg"));
console.log(foo("xxxxx foo arg"));
console.log("xxxxx");
console.log("xxxxx");
console.log("xxxxx");
console.log("xxxxx");
console.log("xxxxx");
console.log("xxxxx");


window.xx = foo("xxxxx foo arg outside");
window.xx = foo("xxxxx foo arg outside");
window.xx = foo("xxxxx foo arg outside");
