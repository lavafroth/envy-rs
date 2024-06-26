// Translated the go code at https://en.wikibooks.org/wiki/Algorithm_Implementation/Strings/Longest_common_substring
pub fn longest_common<'a>(a: &'a str, b: &'a str) -> &'a str {
    let mut dp = vec![vec![0; b.len()]; a.len()];
    let mut longest: usize = 0;
    let mut x_longest: usize = 0;
    for (x, b1) in a.char_indices() {
        for (y, b2) in b.char_indices() {
            if b1 != b2 {
                continue;
            }
            dp[x][y] = if x == 0 || y == 0 {
                1
            } else {
                dp[x - 1][y - 1] + 1
            };
            if dp[x][y] > longest {
                longest = dp[x][y];
                x_longest = x;
            }
        }
    }
    x_longest += 1;
    &a[x_longest - longest..x_longest]
}
