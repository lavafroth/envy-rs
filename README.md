# envy-rs

Generate obfuscated Windows PowerShell payloads that resolve to paths by globbing environment variables.

This is a rewrite of my Golang project Envy in pure Rust. Basically the
same thing without the race conditions.

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

## Performance

Benchmarks measured using hyperfine show that the Rust version
performs around 1.74 times faster than the Golang version.

```
hyperfine -N --runs 20 --warmup 3 "./envy-rs '.exe'" "./envy '.exe'"
Benchmark 1: ./envy-rs '.exe'
  Time (mean ± σ):       5.6 ms ±   0.5 ms    [User: 7.9 ms, System: 1.8 ms]
  Range (min … max):     5.1 ms …   7.1 ms    20 runs
 
Benchmark 2: ./envy '.exe'
  Time (mean ± σ):       9.8 ms ±   0.7 ms    [User: 12.4 ms, System: 2.3 ms]
  Range (min … max):     8.6 ms …  11.7 ms    20 runs
 
Summary
  './envy-rs '.exe'' ran
    1.74 ± 0.20 times faster than './envy '.exe''
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
