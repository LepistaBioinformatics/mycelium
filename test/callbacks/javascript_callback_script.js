#!/usr/bin/env node

const args = process.argv.slice(2);
const contextIndex = args.indexOf('--context');

if (contextIndex === -1 || contextIndex + 1 >= args.length) {
    console.error('Error: --context argument is required');
    process.exit(1);
}

// Parse context JSON (NO PAYLOAD INCLUDED)
const context = JSON.parse(args[contextIndex + 1]);

console.log('JAVASCRIPT CALLBACK SCRIPT EXECUTED');
console.log('=== Response Metadata ===');
console.log(`Status Code: ${context.status_code}`);
console.log(`Duration: ${context.duration_ms}ms`);
console.log(`Method: ${context.method}`);
console.log(`Path: ${context.upstream_path}`);
console.log(`Downstream: ${context.downstream_url}`);

// Access headers
console.log('\n=== Headers ===');
for (const [key, value] of Object.entries(context.headers)) {
    console.log(`${key}: ${value}`);
}

// Custom logic
if (context.status_code >= 500) {
    console.error('❌ Server error detected!');
}

if (context.duration_ms > 1000) {
    console.warn('⚠️  Slow response detected!');
}

console.log('✅ Callback completed');
