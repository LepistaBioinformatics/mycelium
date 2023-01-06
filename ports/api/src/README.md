
## Usage

The public `lib` includes a `extract_profile` function that needs the previous
configuration of the `TOKENS_VALIDATION_PATH` environment variable to work
correctly. Then export it like:

```bash
export TOKENS_VALIDATION_PATH="http://localhost:8081/service/tokens/"
```
