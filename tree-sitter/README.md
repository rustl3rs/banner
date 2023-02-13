build the dockerfile by using `docker-compose up --build`


to hack on the grammar;
```
docker run --rm -ti -v $(pwd)/tree-sitter-banner:/home/node/src/ rustl3rs/tree-sitter-builder bash
```

you might need to just find your way into the correct directory;
```
cd ~/src
```

you'll need 2 commands
```bash
tree-sitter generate
tree-sitter parse example-file
```