# Changelog

## [3.9.0](https://github.com/dallay/corvus/compare/rook-v3.8.2...rook-v3.9.0) (2026-05-06)


### Features

* **rook:** operational parity – undercover mode, debug diagnostics, multi-provider routing controls ([355cc75](https://github.com/dallay/corvus/commit/355cc7577bcb8d10e30e4a7441770dcab9b00350))
* **rook:** operational parity – undercover mode, debug diagnostics, multi-provider routing controls ([6ad9a2b](https://github.com/dallay/corvus/commit/6ad9a2b6b3b29725b5e01f81575bef7885c53567)), closes [#538](https://github.com/dallay/corvus/issues/538)


### Bug Fixes

* address remaining SonarCloud maintainability issues ([20ad14a](https://github.com/dallay/corvus/commit/20ad14af2d675397910afc45bb758cd3d89fe973))
* address remaining SonarCloud review comments ([d5265f3](https://github.com/dallay/corvus/commit/d5265f357bc4f5d985bb8adce37e1e9e58251163))
* address SonarCloud maintainability issues ([d8a8d32](https://github.com/dallay/corvus/commit/d8a8d320c39245ac5a6bf576b2360a7c7742a7d9))
* address SonarCloud quality gate issues ([fd854d0](https://github.com/dallay/corvus/commit/fd854d0ddb42d5670567e34d59aea6248ff22dc9))
* address SonarCloud quality gate issues ([de98462](https://github.com/dallay/corvus/commit/de98462584c8eb3e26e7f47bf3adb4d561ce219a))
* address SonarCloud review feedback ([a3203d1](https://github.com/dallay/corvus/commit/a3203d1ce3666f45862c9fe6949904c0f497a11c))
* **rook:** address review findings from PR [#792](https://github.com/dallay/corvus/issues/792) ([6daf45c](https://github.com/dallay/corvus/commit/6daf45cc6d291cf5474fa014e62346c305c13425))
* update dependencies and improve error handling in various modules ([3237e29](https://github.com/dallay/corvus/commit/3237e29b9216a925568109f9de287417c4c9318c))

## [3.8.2](https://github.com/dallay/corvus/compare/rook-v3.8.1...rook-v3.8.2) (2026-05-05)


### Bug Fixes

* **release:** add npm manifest sync post release-please ([a4ee895](https://github.com/dallay/corvus/commit/a4ee895140b0d511040d3282bc50bb1de1bd0666))

## [3.8.1](https://github.com/dallay/corvus/compare/rook-v3.8.0...rook-v3.8.1) (2026-05-05)


### Bug Fixes

* address code quality review findings ([3f88687](https://github.com/dallay/corvus/commit/3f88687d028cc61cd7ce98177d2401d5a36746de))
* address follow-up review findings ([f55c51b](https://github.com/dallay/corvus/commit/f55c51b238eb508c326337e057271f3cc1f5289f))
* remediate sonar quality issues ([56da8f1](https://github.com/dallay/corvus/commit/56da8f1d15db7c45f3cb29e95952284289a8541d))

## [3.8.0](https://github.com/dallay/corvus/compare/rook-v3.7.0...rook-v3.8.0) (2026-05-05)


### Features

* **coordinator:** add foundational components for coordinator functionality and update timeout durations ([6961e8c](https://github.com/dallay/corvus/commit/6961e8c40b42611012f4d65474d192790ce0a563))
* Enhance workflow permissions, fix detekt alerts, and add Rook dashboard ([e43fc1a](https://github.com/dallay/corvus/commit/e43fc1a9a5b1c0c6ce53ef102832df1ddad1c713))
* implement supervised child lifecycle for Track 4 Slice 2 ([92d591c](https://github.com/dallay/corvus/commit/92d591c1168ba0e1a0833d7a1d6cb52c6b21f250))
* introduce in-process coordinator foundations for Track 4 orchestration ([29bf8c3](https://github.com/dallay/corvus/commit/29bf8c3caffd77c6a99a89a5efd43888039e61d5))
* **orchestration:** add mailbox-backed slice 3 delivery ([0f811dc](https://github.com/dallay/corvus/commit/0f811dcfc71bc8bbc871be13fba6ef6a2b88a51c))
* Persist Rook provider account health state ([930700f](https://github.com/dallay/corvus/commit/930700f7752b4a78f6fb52cd12198080d7b67216))
* **rook:** add admin API for operator management ([0a281a0](https://github.com/dallay/corvus/commit/0a281a048586f9ef86cb298a957581b505f5fa0c))
* **rook:** add admin audit trail and fix repo build ([9f3529e](https://github.com/dallay/corvus/commit/9f3529e12f22e2eb716c984deed9637c68325656)), closes [#599](https://github.com/dallay/corvus/issues/599)
* **rook:** add config baseline, doctor, and health endpoints ([4338801](https://github.com/dallay/corvus/commit/43388015e0df1664220dc5bad2765a51c3b6a318))
* **rook:** add distribution channels and release wiring ([f2121c1](https://github.com/dallay/corvus/commit/f2121c1c120f62bc0f9c61fd7ac9594645c1aac3))
* **rook:** add distribution channels and release wiring ([8b1dce2](https://github.com/dallay/corvus/commit/8b1dce28ada851f4f7ce60c5bcc6230779812f4b))
* **rook:** add embedded operator dashboard flows ([c51c8c1](https://github.com/dallay/corvus/commit/c51c8c1b2b09a79207271bb95d8568e4e17b3a86))
* **rook:** add embedded operator dashboard flows ([a98c315](https://github.com/dallay/corvus/commit/a98c31519951b78a6eaa78ac497879a770be0a22))
* **rook:** add gateway usage accounting ([8381b6b](https://github.com/dallay/corvus/commit/8381b6bee983f3aaf4c20928aa3db4a7eb03aae7))
* **rook:** add gateway usage accounting ([a786870](https://github.com/dallay/corvus/commit/a78687065858f2d2bff1d03381fc8afd9b57c45d))
* **rook:** add OpenAI-compatible gateway endpoints ([41d7539](https://github.com/dallay/corvus/commit/41d75398de62c915bb13e4d3864c463e73b75457))
* **rook:** add pools routes and health dashboard flows ([1add3fa](https://github.com/dallay/corvus/commit/1add3fa6f23b5274534dcc3b62d50d1659a08b0d))
* **rook:** add principal-aware rate limiting ([038a96e](https://github.com/dallay/corvus/commit/038a96ef3226069bfe3415da8c5744fbf323db45))
* **rook:** add principal-aware rate limiting with pruning ([8bd0024](https://github.com/dallay/corvus/commit/8bd0024caf2fc84111cb9268f1d224ab8df37b51))
* **rook:** add production observability metrics ([ef845e1](https://github.com/dallay/corvus/commit/ef845e16b8097df7568064d555faa24bccdb58f0))
* **rook:** add production observability metrics ([0583815](https://github.com/dallay/corvus/commit/0583815e928f0673378670f1f850cb6bb1f522ef))
* **rook:** add production readiness operations ([2ae73ba](https://github.com/dallay/corvus/commit/2ae73ba36f2a2db0d94a98f31292bf45c143c0cc))
* **rook:** add production readiness operations ([4ea27bd](https://github.com/dallay/corvus/commit/4ea27bde0ceea9417838ce83c486e7aae3988f0c))
* **rook:** add Prometheus observability metrics ([d21fc8a](https://github.com/dallay/corvus/commit/d21fc8a5f7e037048a3825075e99bfddd476ff41))
* **rook:** add Prometheus observability metrics ([fe32883](https://github.com/dallay/corvus/commit/fe32883904711c1fa75fc00b505f792d81881b8d))
* **rook:** add SQLite persistence for ProviderAccount, ProviderPool, ModelRoute, and RoutingPolicy ([c6d1b82](https://github.com/dallay/corvus/commit/c6d1b82adf01cf1359c4dbea00c239ea017b7a44))
* **rook:** add SQLite persistence for ProviderAccount, ProviderPool, ModelRoute, and RoutingPolicy ([b99530a](https://github.com/dallay/corvus/commit/b99530a811775dc9b55d93f5f92d08b7c02e54ce)), closes [#583](https://github.com/dallay/corvus/issues/583)
* **rook:** add upstream resilience controls ([5e11013](https://github.com/dallay/corvus/commit/5e1101318473c7ec8296927fa411e5befbdb5c4a))
* **rook:** add upstream resilience controls ([5978f5a](https://github.com/dallay/corvus/commit/5978f5a8b499e78132ebdc7b8b540d1ea8234796))
* **rook:** build registry services for account, pool, route, and settings ([ec1ee5f](https://github.com/dallay/corvus/commit/ec1ee5f2f12ad505b8a6db5fdbc34c75ba08823e))
* **rook:** build registry services for account, pool, route, and settings ([9bc818e](https://github.com/dallay/corvus/commit/9bc818e147751735deda12ea83e5187d9116cc3f)), closes [#584](https://github.com/dallay/corvus/issues/584)
* **rook:** document operational health probes ([133d341](https://github.com/dallay/corvus/commit/133d341c6341b4270c33b3ccdf631bb29802e2f3))
* **rook:** document operational health probes ([4672c6b](https://github.com/dallay/corvus/commit/4672c6ba09b401ae8b18223430b75a17b245d0b8))
* **rook:** embed dashboard assets and coordinate single-binary startup flows ([264bfa1](https://github.com/dallay/corvus/commit/264bfa19929d2e60af7cf36631431a4d94bc91de))
* **rook:** embed dashboard assets and coordinate single-binary startup flows ([4a7cba2](https://github.com/dallay/corvus/commit/4a7cba29a1327c4dd0b5d103fc24183ed12cdc67)), closes [#582](https://github.com/dallay/corvus/issues/582)
* **rook:** finalize tui setup and troubleshooting boundary ([b930dae](https://github.com/dallay/corvus/commit/b930dae7ae09a381f14d71185e5841cd124986b4)), closes [#597](https://github.com/dallay/corvus/issues/597)
* **rook:** harden gateway transport and chat delivery ([93efa55](https://github.com/dallay/corvus/commit/93efa5581cd971ab9241416671ecf4dc15953ac5))
* **rook:** harden security defaults and secret boundaries ([dadc02d](https://github.com/dallay/corvus/commit/dadc02d421194dd907728294ffe51387687a3f80)), closes [#598](https://github.com/dallay/corvus/issues/598)
* **rook:** implement operator dashboard and tui route inspection slices ([6a340d9](https://github.com/dallay/corvus/commit/6a340d92e9427cdb0d9e6ef1b4d23581ec20e379))
* **rook:** implement routing engine with strategy dispatch and fallback chains ([d634ae1](https://github.com/dallay/corvus/commit/d634ae16e3a95cece392695c3804a4a1f28be45c))
* **rook:** implement routing engine with strategy dispatch and fallback chains ([9a96954](https://github.com/dallay/corvus/commit/9a9695458daf90ff8d62ff5a7499f990314085e6)), closes [#586](https://github.com/dallay/corvus/issues/586)
* **rook:** implement shared domain services for accounts, pools, routes, and health ([08e3bc8](https://github.com/dallay/corvus/commit/08e3bc877dea15666f29d305ee72f1193e8367cb))
* **rook:** implement shared domain services for accounts, pools, routes, and health ([7b763f8](https://github.com/dallay/corvus/commit/7b763f8132206fb6a5e640155c400826e22c2383)), closes [#581](https://github.com/dallay/corvus/issues/581)
* **rook:** persist provider account health state ([0932893](https://github.com/dallay/corvus/commit/0932893e221918ae2fd2e49561ff152204c4afd0))
* **rook:** scaffold package layout with domain types, CLI, and module stubs ([a9ee3a7](https://github.com/dallay/corvus/commit/a9ee3a73575a0f60847eb5c67460647f3e519b23))
* **rook:** scaffold package layout with domain types, CLI, and module stubs ([c723f32](https://github.com/dallay/corvus/commit/c723f325fcd2ec98d1ffa6b009a4715d771caba1)), closes [#580](https://github.com/dallay/corvus/issues/580)


### Bug Fixes

* address sonar quality findings ([ea6dd3a](https://github.com/dallay/corvus/commit/ea6dd3ad623bd0ddff4be93a80c8bdb5d5545251))
* address sonar quality findings ([7c73d95](https://github.com/dallay/corvus/commit/7c73d95419c492e9cfe242e339dea4a3622bb15e))
* apply CodeRabbit auto-fixes ([895527c](https://github.com/dallay/corvus/commit/895527c3f97eda5d7eba6d7fd36f6a724918e541))
* apply CodeRabbit auto-fixes ([445a2d0](https://github.com/dallay/corvus/commit/445a2d03f2a428960cc473a466fef954593f71c1))
* apply CodeRabbit auto-fixes ([2fd8667](https://github.com/dallay/corvus/commit/2fd86674ffd1f6f5a4ca0d1b230d499526bd7295))
* resolve detekt code scanning alerts in ChatComponents, MobileRuntimeCoordinator ([1eb6a9e](https://github.com/dallay/corvus/commit/1eb6a9e56810a51fd8bfb6ad9d52371b114ebe0a))
* rook 583 apply ([721302f](https://github.com/dallay/corvus/commit/721302f09b8967fb7926a86c3c485cbedc4e4a66))
* **rook:** add config export and env overrides ([e2d1325](https://github.com/dallay/corvus/commit/e2d1325fc00e6266f9f92b6d76d096f675d91ccc))
* **rook:** add config export and env overrides ([b166398](https://github.com/dallay/corvus/commit/b166398282dd30a26a850137004df36f4a5a4ef5))
* **rook:** add ID newtype accessors and ProviderVendor near-miss deserialization ([40315f3](https://github.com/dallay/corvus/commit/40315f328152c22ec591a00c7fb1be3abe16631f))
* **rook:** address code review findings for routing engine ([b2d551c](https://github.com/dallay/corvus/commit/b2d551c911e1166b90ac9a6ee206d56ed072d935)), closes [#586](https://github.com/dallay/corvus/issues/586)
* **rook:** address inline review findings for routing engine ([5dad006](https://github.com/dallay/corvus/commit/5dad006f63281ebf5532423294b60059155a2087))
* **rook:** address observability review findings ([cfdeff2](https://github.com/dallay/corvus/commit/cfdeff2e87befdef500e5e16ee539b4631f60592))
* **rook:** address PR [#605](https://github.com/dallay/corvus/issues/605) review comments ([e19f2a5](https://github.com/dallay/corvus/commit/e19f2a51b8a11f48fa08b370e2a6c27c3544641a))
* **rook:** address PR review findings ([8aaaf6a](https://github.com/dallay/corvus/commit/8aaaf6a23c600db4eabaa39c0fc3ed89e93c2734))
* **rook:** address production readiness review ([b152a04](https://github.com/dallay/corvus/commit/b152a04d6e1aa613a5a4d8a8d7f50d1b9cdb524e))
* **rook:** address review findings ([14e1721](https://github.com/dallay/corvus/commit/14e1721ccdfa1ac2ad6105bb056133058d007f84))
* **rook:** address usage accounting review feedback ([a532f60](https://github.com/dallay/corvus/commit/a532f601463c07ab18c1debf57328fcf291c12b2))
* **rook:** address workflow and packaging review feedback ([5a196d5](https://github.com/dallay/corvus/commit/5a196d5b53bac665c85ff115984051068abee281))
* **rook:** align admin probes and config export ([4cc7a93](https://github.com/dallay/corvus/commit/4cc7a93e87ea73f9d801f8f6bc43da4dc0da07e4))
* **rook:** clean up merge conflict files, re-apply fixes ([192cdb3](https://github.com/dallay/corvus/commit/192cdb3b7ade06ab2911042e870d7260eda6fdc4))
* **rook:** fix ProviderVendor::Other serde and add serialization tests ([4eaeac2](https://github.com/dallay/corvus/commit/4eaeac2150c9a18df7469996d62b219eecee7ef9))
* **rook:** harden admin API error and integrity handling ([fdd13c2](https://github.com/dallay/corvus/commit/fdd13c2d13a621f57dc041d698c44fe45d8903b9))
* **rook:** harden gateway secret handling and startup wiring ([5ddeb41](https://github.com/dallay/corvus/commit/5ddeb4138287531daeab472fcba78b2a77e05e35))
* **rook:** harden observability and diagnostics ([24ab684](https://github.com/dallay/corvus/commit/24ab6847b792ad91b20b0c361435105ff37689f7))
* **rook:** implement operational doctor diagnostics ([6e92609](https://github.com/dallay/corvus/commit/6e926092475608c38a2506ed03ce27056199a8d1))
* **rook:** implement operational doctor diagnostics ([a5184e5](https://github.com/dallay/corvus/commit/a5184e5c8658213fbedf1e4af7161a2075bcd6a6))
* **security:** reject quoted direct paths ([f497653](https://github.com/dallay/corvus/commit/f4976535087bcc04af096b67c1207018e6fc3c45))

## [0.2.0](https://github.com/dallay/corvus/compare/rook-v0.1.0...rook-v0.2.0) (2026-05-04)


### Features

* Persist Rook provider account health state ([930700f](https://github.com/dallay/corvus/commit/930700f7752b4a78f6fb52cd12198080d7b67216))
* **rook:** add gateway usage accounting ([8381b6b](https://github.com/dallay/corvus/commit/8381b6bee983f3aaf4c20928aa3db4a7eb03aae7))
* **rook:** add gateway usage accounting ([a786870](https://github.com/dallay/corvus/commit/a78687065858f2d2bff1d03381fc8afd9b57c45d))
* **rook:** add principal-aware rate limiting ([038a96e](https://github.com/dallay/corvus/commit/038a96ef3226069bfe3415da8c5744fbf323db45))
* **rook:** add principal-aware rate limiting with pruning ([8bd0024](https://github.com/dallay/corvus/commit/8bd0024caf2fc84111cb9268f1d224ab8df37b51))
* **rook:** add production readiness operations ([2ae73ba](https://github.com/dallay/corvus/commit/2ae73ba36f2a2db0d94a98f31292bf45c143c0cc))
* **rook:** add production readiness operations ([4ea27bd](https://github.com/dallay/corvus/commit/4ea27bde0ceea9417838ce83c486e7aae3988f0c))
* **rook:** add upstream resilience controls ([5e11013](https://github.com/dallay/corvus/commit/5e1101318473c7ec8296927fa411e5befbdb5c4a))
* **rook:** add upstream resilience controls ([5978f5a](https://github.com/dallay/corvus/commit/5978f5a8b499e78132ebdc7b8b540d1ea8234796))
* **rook:** document operational health probes ([133d341](https://github.com/dallay/corvus/commit/133d341c6341b4270c33b3ccdf631bb29802e2f3))
* **rook:** document operational health probes ([4672c6b](https://github.com/dallay/corvus/commit/4672c6ba09b401ae8b18223430b75a17b245d0b8))
* **rook:** persist provider account health state ([0932893](https://github.com/dallay/corvus/commit/0932893e221918ae2fd2e49561ff152204c4afd0))


### Bug Fixes

* address sonar quality findings ([ea6dd3a](https://github.com/dallay/corvus/commit/ea6dd3ad623bd0ddff4be93a80c8bdb5d5545251))
* address sonar quality findings ([7c73d95](https://github.com/dallay/corvus/commit/7c73d95419c492e9cfe242e339dea4a3622bb15e))
* resolve detekt code scanning alerts in ChatComponents, MobileRuntimeCoordinator ([1eb6a9e](https://github.com/dallay/corvus/commit/1eb6a9e56810a51fd8bfb6ad9d52371b114ebe0a))
* **rook:** address production readiness review ([b152a04](https://github.com/dallay/corvus/commit/b152a04d6e1aa613a5a4d8a8d7f50d1b9cdb524e))
* **rook:** address usage accounting review feedback ([a532f60](https://github.com/dallay/corvus/commit/a532f601463c07ab18c1debf57328fcf291c12b2))
* **security:** reject quoted direct paths ([f497653](https://github.com/dallay/corvus/commit/f4976535087bcc04af096b67c1207018e6fc3c45))

## 0.1.0 (2026-04-29)


### Features

* **coordinator:** add foundational components for coordinator functionality and update timeout durations ([6961e8c](https://github.com/dallay/corvus/commit/6961e8c40b42611012f4d65474d192790ce0a563))
* Enhance workflow permissions, fix detekt alerts, and add Rook dashboard ([e43fc1a](https://github.com/dallay/corvus/commit/e43fc1a9a5b1c0c6ce53ef102832df1ddad1c713))
* implement supervised child lifecycle for Track 4 Slice 2 ([92d591c](https://github.com/dallay/corvus/commit/92d591c1168ba0e1a0833d7a1d6cb52c6b21f250))
* introduce in-process coordinator foundations for Track 4 orchestration ([29bf8c3](https://github.com/dallay/corvus/commit/29bf8c3caffd77c6a99a89a5efd43888039e61d5))
* **orchestration:** add mailbox-backed slice 3 delivery ([0f811dc](https://github.com/dallay/corvus/commit/0f811dcfc71bc8bbc871be13fba6ef6a2b88a51c))
* **rook:** add admin API for operator management ([0a281a0](https://github.com/dallay/corvus/commit/0a281a048586f9ef86cb298a957581b505f5fa0c))
* **rook:** add admin audit trail and fix repo build ([9f3529e](https://github.com/dallay/corvus/commit/9f3529e12f22e2eb716c984deed9637c68325656)), closes [#599](https://github.com/dallay/corvus/issues/599)
* **rook:** add config baseline, doctor, and health endpoints ([4338801](https://github.com/dallay/corvus/commit/43388015e0df1664220dc5bad2765a51c3b6a318))
* **rook:** add distribution channels and release wiring ([f2121c1](https://github.com/dallay/corvus/commit/f2121c1c120f62bc0f9c61fd7ac9594645c1aac3))
* **rook:** add distribution channels and release wiring ([8b1dce2](https://github.com/dallay/corvus/commit/8b1dce28ada851f4f7ce60c5bcc6230779812f4b))
* **rook:** add embedded operator dashboard flows ([c51c8c1](https://github.com/dallay/corvus/commit/c51c8c1b2b09a79207271bb95d8568e4e17b3a86))
* **rook:** add embedded operator dashboard flows ([a98c315](https://github.com/dallay/corvus/commit/a98c31519951b78a6eaa78ac497879a770be0a22))
* **rook:** add OpenAI-compatible gateway endpoints ([41d7539](https://github.com/dallay/corvus/commit/41d75398de62c915bb13e4d3864c463e73b75457))
* **rook:** add pools routes and health dashboard flows ([1add3fa](https://github.com/dallay/corvus/commit/1add3fa6f23b5274534dcc3b62d50d1659a08b0d))
* **rook:** add production observability metrics ([ef845e1](https://github.com/dallay/corvus/commit/ef845e16b8097df7568064d555faa24bccdb58f0))
* **rook:** add production observability metrics ([0583815](https://github.com/dallay/corvus/commit/0583815e928f0673378670f1f850cb6bb1f522ef))
* **rook:** add Prometheus observability metrics ([d21fc8a](https://github.com/dallay/corvus/commit/d21fc8a5f7e037048a3825075e99bfddd476ff41))
* **rook:** add Prometheus observability metrics ([fe32883](https://github.com/dallay/corvus/commit/fe32883904711c1fa75fc00b505f792d81881b8d))
* **rook:** add SQLite persistence for ProviderAccount, ProviderPool, ModelRoute, and RoutingPolicy ([c6d1b82](https://github.com/dallay/corvus/commit/c6d1b82adf01cf1359c4dbea00c239ea017b7a44))
* **rook:** add SQLite persistence for ProviderAccount, ProviderPool, ModelRoute, and RoutingPolicy ([b99530a](https://github.com/dallay/corvus/commit/b99530a811775dc9b55d93f5f92d08b7c02e54ce)), closes [#583](https://github.com/dallay/corvus/issues/583)
* **rook:** build registry services for account, pool, route, and settings ([ec1ee5f](https://github.com/dallay/corvus/commit/ec1ee5f2f12ad505b8a6db5fdbc34c75ba08823e))
* **rook:** build registry services for account, pool, route, and settings ([9bc818e](https://github.com/dallay/corvus/commit/9bc818e147751735deda12ea83e5187d9116cc3f)), closes [#584](https://github.com/dallay/corvus/issues/584)
* **rook:** embed dashboard assets and coordinate single-binary startup flows ([264bfa1](https://github.com/dallay/corvus/commit/264bfa19929d2e60af7cf36631431a4d94bc91de))
* **rook:** embed dashboard assets and coordinate single-binary startup flows ([4a7cba2](https://github.com/dallay/corvus/commit/4a7cba29a1327c4dd0b5d103fc24183ed12cdc67)), closes [#582](https://github.com/dallay/corvus/issues/582)
* **rook:** finalize tui setup and troubleshooting boundary ([b930dae](https://github.com/dallay/corvus/commit/b930dae7ae09a381f14d71185e5841cd124986b4)), closes [#597](https://github.com/dallay/corvus/issues/597)
* **rook:** harden gateway transport and chat delivery ([93efa55](https://github.com/dallay/corvus/commit/93efa5581cd971ab9241416671ecf4dc15953ac5))
* **rook:** harden security defaults and secret boundaries ([dadc02d](https://github.com/dallay/corvus/commit/dadc02d421194dd907728294ffe51387687a3f80)), closes [#598](https://github.com/dallay/corvus/issues/598)
* **rook:** implement operator dashboard and tui route inspection slices ([6a340d9](https://github.com/dallay/corvus/commit/6a340d92e9427cdb0d9e6ef1b4d23581ec20e379))
* **rook:** implement routing engine with strategy dispatch and fallback chains ([d634ae1](https://github.com/dallay/corvus/commit/d634ae16e3a95cece392695c3804a4a1f28be45c))
* **rook:** implement routing engine with strategy dispatch and fallback chains ([9a96954](https://github.com/dallay/corvus/commit/9a9695458daf90ff8d62ff5a7499f990314085e6)), closes [#586](https://github.com/dallay/corvus/issues/586)
* **rook:** implement shared domain services for accounts, pools, routes, and health ([08e3bc8](https://github.com/dallay/corvus/commit/08e3bc877dea15666f29d305ee72f1193e8367cb))
* **rook:** implement shared domain services for accounts, pools, routes, and health ([7b763f8](https://github.com/dallay/corvus/commit/7b763f8132206fb6a5e640155c400826e22c2383)), closes [#581](https://github.com/dallay/corvus/issues/581)
* **rook:** scaffold package layout with domain types, CLI, and module stubs ([a9ee3a7](https://github.com/dallay/corvus/commit/a9ee3a73575a0f60847eb5c67460647f3e519b23))
* **rook:** scaffold package layout with domain types, CLI, and module stubs ([c723f32](https://github.com/dallay/corvus/commit/c723f325fcd2ec98d1ffa6b009a4715d771caba1)), closes [#580](https://github.com/dallay/corvus/issues/580)


### Bug Fixes

* apply CodeRabbit auto-fixes ([895527c](https://github.com/dallay/corvus/commit/895527c3f97eda5d7eba6d7fd36f6a724918e541))
* apply CodeRabbit auto-fixes ([445a2d0](https://github.com/dallay/corvus/commit/445a2d03f2a428960cc473a466fef954593f71c1))
* apply CodeRabbit auto-fixes ([2fd8667](https://github.com/dallay/corvus/commit/2fd86674ffd1f6f5a4ca0d1b230d499526bd7295))
* rook 583 apply ([721302f](https://github.com/dallay/corvus/commit/721302f09b8967fb7926a86c3c485cbedc4e4a66))
* **rook:** add config export and env overrides ([e2d1325](https://github.com/dallay/corvus/commit/e2d1325fc00e6266f9f92b6d76d096f675d91ccc))
* **rook:** add config export and env overrides ([b166398](https://github.com/dallay/corvus/commit/b166398282dd30a26a850137004df36f4a5a4ef5))
* **rook:** add ID newtype accessors and ProviderVendor near-miss deserialization ([40315f3](https://github.com/dallay/corvus/commit/40315f328152c22ec591a00c7fb1be3abe16631f))
* **rook:** address code review findings for routing engine ([b2d551c](https://github.com/dallay/corvus/commit/b2d551c911e1166b90ac9a6ee206d56ed072d935)), closes [#586](https://github.com/dallay/corvus/issues/586)
* **rook:** address inline review findings for routing engine ([5dad006](https://github.com/dallay/corvus/commit/5dad006f63281ebf5532423294b60059155a2087))
* **rook:** address observability review findings ([cfdeff2](https://github.com/dallay/corvus/commit/cfdeff2e87befdef500e5e16ee539b4631f60592))
* **rook:** address PR [#605](https://github.com/dallay/corvus/issues/605) review comments ([e19f2a5](https://github.com/dallay/corvus/commit/e19f2a51b8a11f48fa08b370e2a6c27c3544641a))
* **rook:** address PR review findings ([8aaaf6a](https://github.com/dallay/corvus/commit/8aaaf6a23c600db4eabaa39c0fc3ed89e93c2734))
* **rook:** address review findings ([14e1721](https://github.com/dallay/corvus/commit/14e1721ccdfa1ac2ad6105bb056133058d007f84))
* **rook:** address workflow and packaging review feedback ([5a196d5](https://github.com/dallay/corvus/commit/5a196d5b53bac665c85ff115984051068abee281))
* **rook:** align admin probes and config export ([4cc7a93](https://github.com/dallay/corvus/commit/4cc7a93e87ea73f9d801f8f6bc43da4dc0da07e4))
* **rook:** clean up merge conflict files, re-apply fixes ([192cdb3](https://github.com/dallay/corvus/commit/192cdb3b7ade06ab2911042e870d7260eda6fdc4))
* **rook:** fix ProviderVendor::Other serde and add serialization tests ([4eaeac2](https://github.com/dallay/corvus/commit/4eaeac2150c9a18df7469996d62b219eecee7ef9))
* **rook:** harden admin API error and integrity handling ([fdd13c2](https://github.com/dallay/corvus/commit/fdd13c2d13a621f57dc041d698c44fe45d8903b9))
* **rook:** harden gateway secret handling and startup wiring ([5ddeb41](https://github.com/dallay/corvus/commit/5ddeb4138287531daeab472fcba78b2a77e05e35))
* **rook:** harden observability and diagnostics ([24ab684](https://github.com/dallay/corvus/commit/24ab6847b792ad91b20b0c361435105ff37689f7))
* **rook:** implement operational doctor diagnostics ([6e92609](https://github.com/dallay/corvus/commit/6e926092475608c38a2506ed03ce27056199a8d1))
* **rook:** implement operational doctor diagnostics ([a5184e5](https://github.com/dallay/corvus/commit/a5184e5c8658213fbedf1e4af7161a2075bcd6a6))
