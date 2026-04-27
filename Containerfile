FROM rust:1-slim

RUN apt-get update && apt-get install -y \
    libc6-dev \
    gcc \
    make \
    &> /dev/null

WORKDIR /app
COPY . .

# Run only tests to verify OS/libc specifics
CMD ["sh", "-c", "cargo test && ./tests/shell_test.sh ./target/debug/asleep"]
