// Translated the go code at https://en.wikibooks.org/wiki/Algorithm_Implementation/Strings/Longest_common_substring
pub fn longest_common<'a>(a: &'a str, b: &'a str) -> &'a str {
    let m = a.len();
    let n = b.len();

    let s1 = a.as_bytes();
    let s2 = b.as_bytes();

    let mut dp = vec![vec![0; n + 1]; m + 1];
    let mut longest: usize = 0;
    let mut x_longest: usize = 0;
    for x in 1..=m {
        for y in 1..=n {
            if s1[x - 1] == s2[y - 1] {
                dp[x][y] = dp[x - 1][y - 1] + 1;
                if dp[x][y] > longest {
                    longest = dp[x][y];
                    x_longest = x;
                }
            }
        }
    }
    &a[x_longest - longest..x_longest]
}
