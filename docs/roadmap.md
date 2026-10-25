# Yukino Roadmap

Priority for implementation.

## Map

- [ ] More targets [Details](roadmap#Targets)
- [ ] MacOS target [Details](<roadmap#MacOS targets>)
- [ ] git init on project creation [Details](<roadmap#Git init>) 
- [ ] ^~^ operator [Details](<roadmap#Operator ^~^>) 

## Details

### Git init
If the user has Git installed, offer to initialize a repository.

```
Init git into project dir? [Y/n]: 
```

Create `yukino/src/components/yn_input.rs`

If the input is empty, select the uppercase letter. Input is triggered by a keypress, not by pressing Enter.

### Operator ^~^


```yukino
if (3^~^2).contains(x) {}
```

Generate array with nums 1-5 (--3++), e.g generate array with int32/float64 elements in range 2 around 3 (symmetrical radius).

Or

```yukino
//                                    !
set a: int32[] = 5^~^6 // -1 0 1 2 3 4 5 6 7 8 9 0 11
set b = 6

if a.contains(b) {} // true
```

> Inclusive.

### Targets

#### WebAssembly

- [ ] wasm32-unknown-unknown
- [ ] wasm32-wasip1
- [ ] wasm32-wasip2
- [ ] wasm32-unknown-emscripten

#### BSD

- [ ] x86_64-unknown-freebsd
- [ ] aarch64-unknown-freebsd
- [ ] x86_64-unknown-openbsd
- [ ] aarch64-unknown-openbsd
- [ ] x86_64-unknown-netbsd
- [ ] aarch64-unknown-netbsd

#### Windows

- [ ] x86_64-pc-windows-msvc

### MacOS targets

- [ ] aarch64-apple-darwin
- [ ] x86_64-apple-darwin
- [ ] x86_64h-apple-darwin
- [ ] arm64e-apple-darwin