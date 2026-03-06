# Les bdd local sont à lancé avec le docker compose ./docker/bdd/docker-compose.yml

cargo llvm-cov --all --html --open -- --test-threads=1
