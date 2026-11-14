# G-mOMonadOS Documentation

This directory contains documentation for the IMSCRIBER Factorizer and related systems.

## 📚 Available Documentation

### User Guides

| Document | Description | Audience | Last Updated |
|----------|-------------|----------|--------------|
| [FAORIZER_USER_GUIDE.md](./FACTORIZER_USER_GUIDE.md) | Comprehensive user guide for the factorizer | Users, Developers | Nov 14, 2026 |
| [FACTORIZER_CHEAT_SHEET.md](./FACTORIZER_CHEAT_SHEET.md) | Quick reference card | Users | Nov 14, 2026 |

### Technical References

For technical architecture and implementation details, see:

- `../ig-docs/resident_factor_synthesis_construction_20261114.md` — Consolidated technical reference for the resident factor production pipeline
- `../ig-docs/factor_synthesis_progress.md` — Execution record and progress tracking
- `../G-mOMonadOS/measurements/anyon_wiring/workbag_floor_receipts_20261114.md` — Floor confirmation receipts
- `../G-mOMonadOS/measurements/anyon_wiring/REBUILD_RECEIPT_20261114.md` — Rebuild receipts

## 🎯 Quick Links

### Get Started

1. **First factorization**:
   ```bash
   vox mpqs 249880503243289860256926205840602687030138007067151469448678503
   ```

2. **With validation**:
   ```bash
   y_divisor factors 249880503243289860256926205840602687030138007067151469448678503
   ```

### Build from Source

```bash
cd G-mOMonadOS
cargo build --release --features hosted
```

### Key Performance Milestones (as of Nov 14, 2026)

| Target | Bits | Time | Producer | Status |
|--------|------|------|----------|--------|
| 208-bit | 208 | 26.23s | mpqs-relation-source | ✅ Verified |
| 208-bit | 208 | 68.58s | mpqs-relation-source | ✅ Verified |
| 224-bit | 224 | 182.67s | mpqs-relation-source | ✅ Verified |
| 256-bit | 256 | 90s bound | - | ⚠️ Timeout |
| 895-bit | 895 | 90s bound | - | ⚠️ Timeout |

**4-minute floor met for 208-bit and 224-bit targets!**

## 📁 Directory Structure

```
G-mOMonadOS/
├── docs/
│   ├── README.md                    # This file
│   ├── FACTORIZER_USER_GUIDE.md      # Full user guide
│   └── FACTORIZER_CHEAT_SHEET.md     # Quick reference
├── src/
│   └── ...                          # Source code
├── measurements/
│   └── ...                          # Execution receipts
└── ...
```

## 🔍 Documentation Map

### For Users

Start with:
1. **[FACTORIZER_CHEAT_SHEET.md](./FACTORIZER_CHEAT_SHEET.md)** — Quick start and common commands
2. **[FACTORIZER_USER_GUIDE.md](./FACTORIZER_USER_GUIDE.md)** — Complete reference

### For Developers

See also:
- **Code documentation**: Inline Rust docs (run `cargo doc --open`)
- **Architecture**: `../ig-docs/resident_factor_synthesis_construction_20261114.md`
- **Progress**: `../ig-docs/factor_synthesis_progress.md`

### For Maintainers

- **Build process**: `../../scripts/build_state_guard.py`
- **Catalog management**: `$HOME/.imscrbgrmr/catalog.json`
- **Project guidelines**: `../../AGENTS.md`

## 📝 Contributing Documentation

When adding documentation:

1. Update the table above
2. Add cross-references to related documents
3. Include version and date
4. Link to source code and receipts where applicable
5. Follow the existing style and structure

## 🎓 Learning Path

### Beginner
1. Read [FACTORIZER_CHEAT_SHEET.md](./FACTORIZER_CHEAT_SHEET.md)
2. Try factoring a small number: `vox mpqs 91`
3. Try factoring a 208-bit number from the receipts

### Intermediate
1. Read [FACTORIZER_USER_GUIDE.md](./FACTORIZER_USER_GUIDE.md)
2. Understand the three-stage pipeline
3. Learn environment variable controls
4. Generate your own test semiprimes

### Advanced
1. Read `resident_factor_synthesis_construction_20261114.md`
2. Study the code in `src/resident_glut.rs`
3. Understand the monadic state management
4. Explore the SIC reconstruction

### Expert
1. Read all receipts in `measurements/`
2. Study the WorkBag rewrite
3. Understand the anyon socket chain
4. Contribute to the next construction phase

## 📊 Status Dashboard

| Metric | Value | Status |
|--------|-------|--------|
| 200-bit floor | 27s | ✅ Met |
| 208-bit floor | 68.58s | ✅ Met |
| 224-bit floor | 182.67s | ✅ Met |
| 256-bit floor | 90s | ⚠️ Bound |
| 895-bit factor | - | ❌ Pending |
| Test suite | 435/449 passing | ✅ Healthy |
| Build time | ~1-2 minutes | ✅ Acceptable |

## 🔗 External References

- **Project root**: `/home/mrnob0dy666/imsgct/`
- **AGENTS.md**: Project-specific instructions and constraints
- **Catalog**: `$HOME/.imscrbgrmr/catalog.json`
- **Lean corpus**: `p4rakernel/p4ramill/Imscribing/`
- **Reference docs**: `ig-docs/`

## 📞 Support

For issues or questions:

1. Check this documentation
2. Review receipts in `measurements/`
3. Consult `AGENTS.md` for project-specific guidance
4. Verify build state: `python3 scripts/build_state_guard.py check`

---

*Last updated: November 14, 2026*
*Maintainer: Lando / IMSCRIBER Team*
