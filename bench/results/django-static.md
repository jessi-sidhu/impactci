# django (selection only)

100 commits. Median 99% of test files selected, mean 87%. Full suite on 13/100 commits. Median analysis time 1.40s.

| test files selected | commits |
|---|---|
| none | 0 |
| under 10% | 13 |
| 10–50% | 0 |
| 50–99% | 74 |
| all | 13 |

| commit | subject | mode | selected | reason |
|---|---|---|---|---|
| `9014ca519c` | Fixed #37418 -- Fixed JSONField/HStoreField crash on deeply nested JSON. | selected | 835/840 |  |
| `1a9fba7a6c` | Fixed #23424 -- Corrected tokenization of verbatim blocks. | selected | 835/840 |  |
| `de6559094e` | Refs #23424 -- Added tests for existing comment and verbatim behavior. | selected | 2/840 |  |
| `31c3e4cf75` | Refs #37372 -- Simplified NULL handling logic in sql.Query.build_filter( | selected | 835/840 |  |
| `f1d11117c6` | Fixed #28219 -- Made UnorderedObjectListWarning skip Django frames. | selected | 835/840 |  |
| `6669d66a79` | Skipped 1-element geometry collection tests on all Oracle versions. | selected | 835/840 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `cf63ff2186` | Fixed #37411 -- Added compatibility for oracledb 26.0.0+. | selected | 835/840 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `677fbf05da` | Refs #37271 -- Updated docs/conf.py for calendar versioning. | selected | 835/840 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `0264b4fc36` | Fixed #37387 -- Made admindocs model index table full width. | full | 840/840 | django/contrib/admin/static/admin/css/base.css isn't Python, tests might read it |
| `a272e554b8` | Fixed GeoQuerySetTest.test_unionagg_tolerance() to accept either merged  | selected | 1/840 |  |
| `bef1986b9a` | Fixed #37118 -- Added variable support to the {% now %} template tag. | selected | 835/840 |  |
| `137b77706e` | Refs CVE-2026-15830 -- Increased coverage for WKT depth check in GEOS li | selected | 1/840 |  |
| `93d7e82b6a` | Fixed #37382, Refs #29303 -- Made @non_atomic_requests recognize async v | selected | 835/840 |  |
| `ce285ce0e7` | Added a 24 hour grace period for automated PR quality checks. | full | 840/840 | .github/workflows/check_pr_quality.yml changes how CI runs tests |
| `1267767ccd` | Fixed passing of pagination data as params to github_request(). | selected | 836/840 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `1b4d021b65` | Fixed #37403 -- Fixed JSONField __in crash on SQLite with Value(None) ex | selected | 835/840 |  |
| `7f48e266a8` | Refs #37297 -- Prevented crash on JSONField __isnull lookups on Oracle w | selected | 835/840 |  |
| `66c40aafa2` | Fixed #37297 -- Prevented crash on JSONField key lookups when an integer | selected | 835/840 |  |
| `135d7c606f` | Allowed full stops inside quotes in commit messages. | full | 840/840 | .github/workflows/check_commit_messages.yml changes how CI runs tests |
| `a269574f1b` | Fixed #37405 -- Fixed crash if sort_keys option is used with YAML serial | selected | 835/840 |  |
| `62ef9d40cf` | Refs CVE-2026-15830 -- Fixed WKB geometry collection limit bypasses. | full | 840/840 | tests/gis_tests/data/geometries.json isn't Python, tests might read it |
| `83cbd21be5` | Fixed CVE-2026-87975 -- Prevented forging actions in model formsets with | selected | 835/840 |  |
| `ebcb13b327` | Fixed CVE-2026-87890 -- Blocked bytes in spatial lookups. | selected | 835/840 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `7ff7fcc050` | Fixed CVE-2026-84429 -- Simplified parse_header_parameters by leveraging | selected | 835/840 |  |
| `c88b304cc2` | Fixed CVE-2026-77050 -- Prevented memory exhaustion in get_supported_lan | selected | 835/840 |  |
| `08e4c0d8e7` | Fixed #37322 -- Avoided transforms on spatial fields with srid=None. | selected | 835/840 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `fabce94374` | Fixed #37377 -- Fixed JSONField __in crash/wrong result for Value() prim | selected | 835/840 |  |
| `8abacfc36d` | Refs #37386 -- Tailored error messages for the delete_confirmation_max_d | full | 840/840 | django/contrib/admin/templates/admin/delete_confirmation.html isn't Python, tests might read it |
| `7d94ee295d` | Fixed #37386 -- Observed delete_confirmation_max_display=0 for inlines. | full | 840/840 | django/contrib/admin/templates/admin/delete_confirmation.html isn't Python, tests might read it |
| `a461af8ce4` | Fixed decimal comparison in RelatedGeoModelTest.test_related_union_aggre | selected | 1/840 |  |
| `7847227a3f` | Fixed #37361 -- Fixed permission renames for chained RenameModel operati | full | 840/840 | tests/auth_tests/operations_migrations/0003_rename_newmodel_to_anothermodel.py isn't an importable module, might be run by tests |
| `3db0898eee` | Fixed #37376 -- Made UUID4() persist data in lowercase on Oracle. | selected | 835/840 |  |
| `80ea222c8a` | Fixed #37167 -- Allowed email.headerregistry.Address objects as email ad | selected | 835/840 |  |
| `91585625a9` | Fixed #37212 -- Improved djangodocs Sphinx extension portability. | full | 840/840 | biome.json isn't Python, tests might read it |
| `50eef95591` | Fixed #37291 -- Prevented tuple lookups from hanging on F() expressions. | selected | 835/840 |  |
| `479ba6d281` | Fixed #37378 -- Removed extra empty breadcrumb in admindocs model index. | full | 840/840 | django/contrib/admindocs/templates/admin_doc/model_index.html isn't Python, tests might read it |
| `2f7f30c320` | Fixed #36496 -- Ensured the parent path to a sqlite file exists. | selected | 835/840 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `9332b163a6` | Refs #37136 -- Fixed test skips for Oracle Test Pilot. | selected | 1/840 |  |
| `dffadf6dfc` | Fixed #37350 -- Made distinct(*fields) resolve aliases containing "__". | selected | 835/840 |  |
| `9e06baf2e4` | Fixed #37066 -- Wrapped async iteration sites in (maybe_)aclosing. | selected | 835/840 |  |
| `a013c821ea` | Fixed #37177 -- Reduced per-middleware context switching under ASGI. | selected | 835/840 |  |
| `703283b5b4` | Fixed #37358 -- Dropped support for GDAL 3.3 and 3.4. | selected | 835/840 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `446d9cf602` | Fixed #37114 -- Dropped support for PostgreSQL 15 and PostGIS 3.2. | full | 840/840 | .github/workflows/check-migrations.yml changes how CI runs tests |
| `82f34e2f72` | Refs #36947 -- Added supports_comments skip to test_alter_generated_fiel | selected | 1/840 |  |
| `935edaa91b` | Fixed #37357 -- Handled out-of-range year in admin date_hierarchy. | selected | 835/840 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `a3f0642f69` | Refs #36947 -- Added missing max_length in GeneratedField test for Oracl | selected | 1/840 |  |
| `581deb9402` | Fixed #37339 -- Allowed ipaddress objects in GenericIPAddressField. | selected | 835/840 |  |
| `6ade625848` | Fixed #37344 -- Enabled FETCH_PEERS batching for instances loaded by sel | selected | 835/840 |  |
| `08ba4d0e0f` | Modeled a safer mechanism for installing a git hook for backports. | selected | 836/840 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `2abf9d2cf8` | Refs #37348 -- Added test for excluding on a nullable GeneratedField. | selected | 1/840 |  |
| `fb32a564cc` | Fixed #37348 -- Reverted "Fixed #36806 -- Added system check for null kw | selected | 835/840 |  |
| `e7f464ecdc` | Refs #37348 -- Reverted "Refs #36806 -- Removed unnecessary null=True fr | selected | 835/840 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `8cbdd4a814` | Fixed #37286 -- Preserved unique constraints when removing db_index. | selected | 835/840 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `64edef37e7` | Fixed #36947 -- Avoided altering column type when only db_comment change | selected | 835/840 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `9cb2cba6fc` | Fixed typo in django/db/backends/mysql/schema.py. | selected | 835/840 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `97c38590a2` | Fixed false positive in func_supports_parameter() for non-keyword args. | selected | 835/840 |  |
| `abb9c04bd6` | Fixed #36629 -- Added missing "Choose all" and "Remove all" buttons in a | full | 840/840 | django/contrib/admin/static/admin/css/responsive.css isn't Python, tests might read it |
| `850e026cbf` | Fixed #36809 -- Allowed customizing EmailValidator per-part validation. | selected | 835/840 |  |
| `fc4eaaabf4` | Fixed #37289 -- Avoided deleting network rasters. | selected | 835/840 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `e7d301b06a` | Fixed Playwright test flakiness in admin collapse filter tests. | selected | 1/840 |  |
| `be6a149de8` | Fixed #37145 -- Made ModelFormSet respect custom Form.add_prefix(). | selected | 835/840 |  |
| `5f0293b3ab` | Fixed #37136 -- Added test skips for Oracle Test Pilot. | selected | 1/840 |  |
| `fee1cf7262` | Refs #37127 -- Added a version-aware sourcefile Sphinx role. | full | 840/840 | tests/sphinx/__init__.py was removed and isn't inside a package, can't tell what used it |
| `b3f4d83aad` | Fixed #37013 -- Deprecated Trunc/Extract in migrations with USE_TZ=True  | selected | 833/838 |  |
| `0398417c50` | Fixed #36523 -- Added django.utils.module_loading.qualname() helper. | selected | 833/838 |  |
| `cc933385e7` | Fixed #37271 -- Added calendar version support to django.utils.version. | selected | 833/838 |  |
| `aca5b3f694` | Refs #37178 -- Deferred django.middleware import in django.utils.depreca | selected | 833/838 |  |
| `026b005f3d` | Refs #37293 -- Updated deprecation docs for calendar-based warning names | selected | 833/838 |  |
| `52abe52bec` | Fixed #37293 -- Renamed RemovedInDjango71Warning to RemovedInDjango2029W | selected | 833/838 |  |
| `0d0e7f1503` | Refs #37293 -- Aliased RemovedInDjango70Warning to RemovedInDjango2028Wa | selected | 833/838 |  |
| `00dca6f097` | Fixed #37222 -- Fixed QuerySet.distinct() crash on duplicated selections | selected | 833/838 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `f14e293cd7` | Fixed #37312, Refs #36605 -- Fixed annotation preservation and key selec | selected | 833/838 |  |
| `0751d5fed1` | Fixed #37311 -- Prevented consumption of rhs iterators in annotation loo | selected | 833/838 |  |
| `6126f5d2fe` | Refs #37309 -- Used _is_pk_set() in the XML serializer. | selected | 833/838 |  |
| `8d1c673466` | Fixed #37309 -- Used _is_pk_set() in Model.__eq__. | selected | 833/838 |  |
| `5babd2e21a` | Fixed #37237 -- Improved RelatedIn lookup preparation performance. | selected | 833/838 |  |
| `60a17fccf4` | Fixed #37300 -- Preserved parent instance hints on custom Prefetch query | selected | 833/838 |  |
| `fe81969e83` | Fixed #37263 -- Fixed changelist __exact search crashes and over-matchin | selected | 833/838 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `73cc09f14f` | Fixed #36945 -- Made .values() and .order_by() resolve FilteredRelation  | selected | 833/838 |  |
| `439b936753` | Added test skip for 1-element GeometryCollections on Oracle 23.9. | selected | 833/838 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `fdfbb711e3` | Fixed #37296 -- Included the invalid view type in path()/re_path() error | selected | 833/838 |  |
| `7576781cc6` | Fixed #37255 -- Avoided relying on implicit SELECT ordering in tests. | selected | 833/838 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `cccc004b46` | Skipped tests involving 1-element GeometryCollections on Oracle 23.9. | selected | 833/838 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `1d50f12929` | Fixed #37258, Refs #36644 -- Documented error when default ordering fiel | selected | 1/838 |  |
| `d992705f9e` | Fixed #37259 -- Restored support for old-signature Model.from_db() overr | selected | 833/838 |  |
| `504d1f12cd` | Refs #37260 -- Added missing skip condition to test_fk_alter_on_delete_d | selected | 1/838 |  |
| `464a5e1844` | Fixed #37154, #37149 -- Switched from Selenium to Playwright for integra | full | 838/838 | .github/workflows/playwright.yml changes how CI runs tests |
| `be6cf83229` | Fixed #37274 -- Allowed transforms in order_by() after alias(). | selected | 834/839 |  |
| `c72f5fb479` | Fixed #37279 -- Rejected null characters in URLValidator. | selected | 834/839 |  |
| `f7610bda78` | Fixed #37250 -- Allowed nesting GEOS GeometryCollection objects. | selected | 834/839 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `a3e6e7d3aa` | Fixed #37275, Refs #24920 -- Fixed SQLite DecimalField conversion withou | selected | 834/839 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `1a001208b0` | Fixed #37278 -- Made QuerySet.totally_ordered understand aliases to pure | selected | 834/839 |  |
| `07d4f69c94` | Fixed #37260 -- Made alterations between Python on_delete options noops. | selected | 834/839 |  |
| `4ee04972e7` | Refs CVE-2026-15307 -- Used a more common lookup in raster lookup tests. | selected | 1/839 |  |
| `febefb175e` | Fixed #37248 -- Skipped unique validation of a dynamic DatabaseDefault e | selected | 834/839 |  |
| `812c08bd4e` | Fixed #37273 -- Allowed F() expressions on the RHS of CompositePK lookup | selected | 834/839 |  |
| `99d56e5cf1` | Fixed #37262 -- Restored rendering of html-safe strings in form media. | selected | 834/839 |  |
| `c6be0bf3bb` | Fixed #37254 -- Added fields.E323 for models referenced by ManyToManyFie | selected | 834/839 |  |
| `082b3df406` | Fixed #37270 -- Fixed incorrect values for second-degree relations in Mo | selected | 834/839 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
| `6df8fe3bc1` | Fixed #24580 -- Tested FK values with __html__() in ModelAdmin.list_disp | selected | 834/839 | django/apps/config.py:111 has a dynamic import, running tests that reach it |
