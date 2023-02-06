# envy-rs

Generate obfuscated Windows PowerShell payloads that resolve to paths by globbing environment variables.

## Installation

```sh
git clone https://github.com/lavafroth/envy-rs.git
cd envy-rs && cargo build --release
```

## Usage

```
envy-rs [OPTIONS] <PATH>
```

Check out all the flags using:

```
envy-rs --help
```

## Where is the Golang tool?

This is a rewrite of my Golang project Envy in pure Rust. The Go tool
has been phased out in favor of this tool because the former used unsafe
concurrency patterns (leading to data races) and ineffective regular
expressions instead of implementing a smaller subset of it for the specific
use case. In terms of performance this version is around 4 times faster than
the go tool. Here are the benchmarks:

```
hyperfine -N --warmup 100 "./envy-rs 'console.exe'" "./envy 'console.exe'"
Benchmark 1: ./envy-rs 'console.exe'
  Time (mean ± σ):      17.0 ms ±   2.2 ms    [User: 22.5 ms, System: 8.8 ms]
  Range (min … max):    14.1 ms …  23.9 ms    191 runs
 
Benchmark 2: ./envy 'console.exe'
  Time (mean ± σ):      92.7 ms ±  15.3 ms    [User: 100.8 ms, System: 16.3 ms]
  Range (min … max):    75.3 ms … 129.0 ms    38 runs
 
Summary
  './envy-rs 'console.exe'' ran
    5.46 ± 1.15 times faster than './envy 'console.exe''
```

## Examples

### Obfuscating a path

```sh
envy-rs 'C:\Windows\System32\calc.exe'
```

```
(("${env:*oms*}"[0..20]-join'')+"alc.exe")
(("${env:coms*}"[0..20]-join'')+"alc.exe")
"c:\"+("${env:o?}"[0..6]-join'')+"\system32\calc.exe"
(("${env:p?t?}"[31..50]-join'')+"calc.exe")
"${env:wi*}\system32\calc.exe"
(("${env:d*}"[0..19]-join'')+"calc.exe")
(("${env:c?m?p*}"[0..20]-join'')+"alc.exe")
--{SNIP}--
```

### Using a target length

By default, the target length for the globbing disabled, meaning
all possible payloads will be displayed. When a target length is
set to `n`, only payloads that are at most `n` characters will
be displayed. 


```sh
envy-rs 'C:\Windows\System32\calc.exe' --target-length 30
```

```
"${env:wi*}\system32\calc.exe"
"${env:w*r}\system32\calc.exe"
"${env:*ir}\system32\calc.exe"
"${env:s*t}\system32\calc.exe"
"${env:*ot}\system32\calc.exe"
--{SNIP}--
```

### Using a custom number of threads

Envy will use 4 threads as a default but this can be modified using
the `-t` or the `--threads` flag. Unlike the golang version, this uses
1:1 thread mapping instead of coroutines.

```sh
envy-rs 'C:\Windows\System32\calc.exe' --threads 6
```

We get the same output, just faster 😉.

### Output generated payloads to a file

Envy will default to standard output for writing the generated payloads
but we can ask it to output to a file by passing a filename to the `-o` or
the `--output` flag.

```sh
envy-rs 'C:\Windows\System32\calc.exe' --output payloads.log
```

### Using a custom environment map

The contents of the environment map in `environment.yaml` will be used by
Envy as a default. A custom environment map can be supplied through the
`--custom-environment-map` flag.

The file must be in the `yaml` format with the entries being the **values** of the
environment variables with the **names of the environment variables** as children of
list.

For example:

```yaml
# This is the value.
'c:'
    # These are the variables names that resolve to it.
    - homedrive
    - systemdrive

```

Once you have your custom environment map, use it with Envy as the following:

```sh
envy-rs 'C:\Windows\System32\calc.exe' --custom-environment-map your_custom_env.yaml
```
