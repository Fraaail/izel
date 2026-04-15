# Sample Applications Tutorial

This suite is designed as a capability atlas for Izel.

## What Changed

The previous set has been replaced with 100 distinct implementations.
They are no longer one-template variations; each file emphasizes different syntax and behavior.

## Recommended Learning Path

1. `001`-`020`: Control flow, numeric modeling, basic diagnostics.
2. `021`-`040`: Dashboarding patterns, modules, shape/scroll modeling.
3. `041`-`060`: Contracts, effects, generics, and trait-like abstractions.
4. `061`-`080`: Witnesses, zones, memory surfaces, iterators/pipelines.
5. `081`-`100`: Macros, async flow/tide syntax, duality demonstrations, capstones.

## Fast Validation

```bash
bash tools/ci/with_llvm_env.sh cargo run -p izel_driver -- sample_applications/013_customer_churn_monitor.iz
```

```bash
for f in sample_applications/[0-9][0-9][0-9]_*.iz; do
  bash tools/ci/with_llvm_env.sh cargo run -p izel_driver -- "$f" || break
done
```
