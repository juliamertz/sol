function printf(format) {}

function fib(n) {
	if (n === 0 || n === 1) {
		return n;
	}
	return fib(n - 1) + fib(n - 2);
}

function main() {
	let result = fib(30);
	printf("Result is %d", result);
}
