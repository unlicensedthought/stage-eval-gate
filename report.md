# Stage evaluation

Backend `stub`, stage model `stub-stage`, judge `stub-judge`, 11 fixtures, 16 configurations.

**Offline stub run: these numbers exercise the harness and are not a measurement.**

## Verdicts

| stage | verdict | target Δ | faithfulness Δ | harm Δ | tokens/turn | $/turn | errors |
|---|---|---|---|---|---|---|---|
| `sycophancy` | **useful** | +3.50 | +0.00 | +0.00 | 421 | 0.000507 | 0 |
| `plain_words` | **cost_little_gain** | +0.42 | +0.00 | +0.00 | 359 | 0.000462 | 0 |
| `one_liner` | **trash** | -2.25 | +0.00 | -1.70 | 286 | 0.000319 | 2 |

## Order notes

- sycophancy and plain_words: order makes no real difference (0.00)
- sycophancy and one_liner: order makes no real difference (0.03)
- plain_words and one_liner: order makes no real difference (0.00)

## Avoid

- sycophancy>one_liner: scores 0.88 below its better stage alone, at more cost
- one_liner>sycophancy: scores 0.91 below its better stage alone, at more cost
- plain_words>one_liner: scores 0.76 below its better stage alone, at more cost
- one_liner>plain_words: scores 0.76 below its better stage alone, at more cost

## Every configuration

| config | composite |
|---|---|
| none | 4.54 |
| sycophancy | 4.96 |
| plain_words | 4.58 |
| one_liner | 3.80 |
| sycophancy>plain_words | 5.00 |
| sycophancy>one_liner | 4.09 |
| plain_words>sycophancy | 5.00 |
| plain_words>one_liner | 3.82 |
| one_liner>sycophancy | 4.05 |
| one_liner>plain_words | 3.82 |
| sycophancy>plain_words>one_liner | 4.11 |
| sycophancy>one_liner>plain_words | 4.11 |
| plain_words>sycophancy>one_liner | 4.11 |
| plain_words>one_liner>sycophancy | 4.08 |
| one_liner>sycophancy>plain_words | 4.08 |
| one_liner>plain_words>sycophancy | 4.08 |

Evaluation cost: 176 apply calls, 8812 judge tokens, 123668 stage tokens (~$0.1458).
