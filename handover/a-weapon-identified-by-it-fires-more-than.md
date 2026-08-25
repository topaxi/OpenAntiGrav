# A weapon identified by "it fires more than one" is not identified

2026-08-11. The first search for the rocket found `0x088675cc` - one projectile per `0.1 s` until a round counter runs out - and it was written up at 68 confidence as the Rocket, which would have shipped a staggered stream instead of a fan. It is almost certainly the **Cannon** (`rounds` and `rate` are its `<Stats>`, and no other weapon's). What caught it was a maintainer who had actually played the game saying the three fly in parallel. The real handler, `Weapon_FireRocket` (`0x0886e104`), makes three literal spawn calls with a `+spread`/`-spread` rotation between them. **The fingerprint that would have settled it immediately was `spread`'s reader**, not the shot count - and `spread` is at rocket-stats `+0x24`.

## Open

- `0x088675cc` is "almost certainly" the Cannon (matches its `rounds`/`rate` `<Stats>`) but not conclusively confirmed.

## Next Steps

No next step named in the original record - read the prose above and decide one.
