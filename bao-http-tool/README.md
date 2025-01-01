# Validate external data using external outboard

## Usage

Create an outboard for an asset that is available via http:

```
❯ bao-http-tool generate --data http://127.0.0.1:3003/370e2b3002be3b38b120f7b3be53da4cf646810e26f8f4a5018247c8188af5b2.data --block-size-log 4
Computing outboard for http://127.0.0.1:3003/370e2b3002be3b38b120f7b3be53da4cf646810e26f8f4a5018247c8188af5b2.data of size 100000000 with block log 4, size 16384
Computed hash: 370e2b3002be3b38b120f7b3be53da4cf646810e26f8f4a5018247c8188af5b2
Writing outboard to 370e2b3002be3b38b120f7b3be53da4cf646810e26f8f4a5018247c8188af5b2.obao4
```

Validate a range of the file:

```
validate --hash 370e2b3002be3b38b120f7b3be53da4cf646810e26f8f4a5018247c8188af5b2 --block-size-log 4 --data http://127.0.0.1:3003/370e2b3002be3b38b120f7b3be53da4cf646810e26f8f4a5018247c8188af5b2.data --outboard 370e2b3002be3b38b120f7b3be53da4cf646810e26f8f4a5018247c8188af5b2.obao4 --range 0..10000000
Size: 100000000
Outboard: 390592
Hash: 370e2b3002be3b38b120f7b3be53da4cf646810e26f8f4a5018247c8188af5b2
Block size: BlockSize(4)
Byte ranges: RangeSet{0..10000000}
Chunk ranges: RangeSet{0..9766}
confirmed that range RangeSet{0..10000000} of http://127.0.0.1:3003/370e2b3002be3b38b120f7b3be53da4cf646810e26f8f4a5018247c8188af5b2.data matches 370e2b3002be3b38b120f7b3be53da4cf646810e26f8f4a5018247c8188af5b2
```

Note that for validation both data **and** outboard can be available via http.