#![allow(dead_code)]

fn n_5_00000(x: i32) -> i32 { x.wrapping_add(6) }
fn n_5_00001(x: i32) -> i32 { x.wrapping_add(7) }
fn n_5_00002(x: i32) -> i32 { x.wrapping_add(8) }
fn n_5_00003(x: i32) -> i32 { x.wrapping_add(9) }
fn n_4_0000(x: i32) -> i32 { n_5_00000(x) + n_5_00001(x) + n_5_00002(x) + n_5_00003(x) }
fn n_5_00010(x: i32) -> i32 { x.wrapping_add(11) }
fn n_5_00011(x: i32) -> i32 { x.wrapping_add(12) }
fn n_5_00012(x: i32) -> i32 { x.wrapping_add(13) }
fn n_5_00013(x: i32) -> i32 { x.wrapping_add(14) }
fn n_4_0001(x: i32) -> i32 { n_5_00010(x) + n_5_00011(x) + n_5_00012(x) + n_5_00013(x) }
fn n_5_00020(x: i32) -> i32 { x.wrapping_add(16) }
fn n_5_00021(x: i32) -> i32 { x.wrapping_add(17) }
fn n_5_00022(x: i32) -> i32 { x.wrapping_add(18) }
fn n_5_00023(x: i32) -> i32 { x.wrapping_add(19) }
fn n_4_0002(x: i32) -> i32 { n_5_00020(x) + n_5_00021(x) + n_5_00022(x) + n_5_00023(x) }
fn n_5_00030(x: i32) -> i32 { x.wrapping_add(21) }
fn n_5_00031(x: i32) -> i32 { x.wrapping_add(22) }
fn n_5_00032(x: i32) -> i32 { x.wrapping_add(23) }
fn n_5_00033(x: i32) -> i32 { x.wrapping_add(24) }
fn n_4_0003(x: i32) -> i32 { n_5_00030(x) + n_5_00031(x) + n_5_00032(x) + n_5_00033(x) }
fn n_3_000(x: i32) -> i32 { n_4_0000(x) + n_4_0001(x) + n_4_0002(x) + n_4_0003(x) }
fn n_5_00100(x: i32) -> i32 { x.wrapping_add(27) }
fn n_5_00101(x: i32) -> i32 { x.wrapping_add(28) }
fn n_5_00102(x: i32) -> i32 { x.wrapping_add(29) }
fn n_5_00103(x: i32) -> i32 { x.wrapping_add(30) }
fn n_4_0010(x: i32) -> i32 { n_5_00100(x) + n_5_00101(x) + n_5_00102(x) + n_5_00103(x) }
fn n_5_00110(x: i32) -> i32 { x.wrapping_add(32) }
fn n_5_00111(x: i32) -> i32 { x.wrapping_add(33) }
fn n_5_00112(x: i32) -> i32 { x.wrapping_add(34) }
fn n_5_00113(x: i32) -> i32 { x.wrapping_add(35) }
fn n_4_0011(x: i32) -> i32 { n_5_00110(x) + n_5_00111(x) + n_5_00112(x) + n_5_00113(x) }
fn n_5_00120(x: i32) -> i32 { x.wrapping_add(37) }
fn n_5_00121(x: i32) -> i32 { x.wrapping_add(38) }
fn n_5_00122(x: i32) -> i32 { x.wrapping_add(39) }
fn n_5_00123(x: i32) -> i32 { x.wrapping_add(40) }
fn n_4_0012(x: i32) -> i32 { n_5_00120(x) + n_5_00121(x) + n_5_00122(x) + n_5_00123(x) }
fn n_5_00130(x: i32) -> i32 { x.wrapping_add(42) }
fn n_5_00131(x: i32) -> i32 { x.wrapping_add(43) }
fn n_5_00132(x: i32) -> i32 { x.wrapping_add(44) }
fn n_5_00133(x: i32) -> i32 { x.wrapping_add(45) }
fn n_4_0013(x: i32) -> i32 { n_5_00130(x) + n_5_00131(x) + n_5_00132(x) + n_5_00133(x) }
fn n_3_001(x: i32) -> i32 { n_4_0010(x) + n_4_0011(x) + n_4_0012(x) + n_4_0013(x) }
fn n_5_00200(x: i32) -> i32 { x.wrapping_add(48) }
fn n_5_00201(x: i32) -> i32 { x.wrapping_add(49) }
fn n_5_00202(x: i32) -> i32 { x.wrapping_add(50) }
fn n_5_00203(x: i32) -> i32 { x.wrapping_add(51) }
fn n_4_0020(x: i32) -> i32 { n_5_00200(x) + n_5_00201(x) + n_5_00202(x) + n_5_00203(x) }
fn n_5_00210(x: i32) -> i32 { x.wrapping_add(53) }
fn n_5_00211(x: i32) -> i32 { x.wrapping_add(54) }
fn n_5_00212(x: i32) -> i32 { x.wrapping_add(55) }
fn n_5_00213(x: i32) -> i32 { x.wrapping_add(56) }
fn n_4_0021(x: i32) -> i32 { n_5_00210(x) + n_5_00211(x) + n_5_00212(x) + n_5_00213(x) }
fn n_5_00220(x: i32) -> i32 { x.wrapping_add(58) }
fn n_5_00221(x: i32) -> i32 { x.wrapping_add(59) }
fn n_5_00222(x: i32) -> i32 { x.wrapping_add(60) }
fn n_5_00223(x: i32) -> i32 { x.wrapping_add(61) }
fn n_4_0022(x: i32) -> i32 { n_5_00220(x) + n_5_00221(x) + n_5_00222(x) + n_5_00223(x) }
fn n_5_00230(x: i32) -> i32 { x.wrapping_add(63) }
fn n_5_00231(x: i32) -> i32 { x.wrapping_add(64) }
fn n_5_00232(x: i32) -> i32 { x.wrapping_add(65) }
fn n_5_00233(x: i32) -> i32 { x.wrapping_add(66) }
fn n_4_0023(x: i32) -> i32 { n_5_00230(x) + n_5_00231(x) + n_5_00232(x) + n_5_00233(x) }
fn n_3_002(x: i32) -> i32 { n_4_0020(x) + n_4_0021(x) + n_4_0022(x) + n_4_0023(x) }
fn n_5_00300(x: i32) -> i32 { x.wrapping_add(69) }
fn n_5_00301(x: i32) -> i32 { x.wrapping_add(70) }
fn n_5_00302(x: i32) -> i32 { x.wrapping_add(71) }
fn n_5_00303(x: i32) -> i32 { x.wrapping_add(72) }
fn n_4_0030(x: i32) -> i32 { n_5_00300(x) + n_5_00301(x) + n_5_00302(x) + n_5_00303(x) }
fn n_5_00310(x: i32) -> i32 { x.wrapping_add(74) }
fn n_5_00311(x: i32) -> i32 { x.wrapping_add(75) }
fn n_5_00312(x: i32) -> i32 { x.wrapping_add(76) }
fn n_5_00313(x: i32) -> i32 { x.wrapping_add(77) }
fn n_4_0031(x: i32) -> i32 { n_5_00310(x) + n_5_00311(x) + n_5_00312(x) + n_5_00313(x) }
fn n_5_00320(x: i32) -> i32 { x.wrapping_add(79) }
fn n_5_00321(x: i32) -> i32 { x.wrapping_add(80) }
fn n_5_00322(x: i32) -> i32 { x.wrapping_add(81) }
fn n_5_00323(x: i32) -> i32 { x.wrapping_add(82) }
fn n_4_0032(x: i32) -> i32 { n_5_00320(x) + n_5_00321(x) + n_5_00322(x) + n_5_00323(x) }
fn n_5_00330(x: i32) -> i32 { x.wrapping_add(84) }
fn n_5_00331(x: i32) -> i32 { x.wrapping_add(85) }
fn n_5_00332(x: i32) -> i32 { x.wrapping_add(86) }
fn n_5_00333(x: i32) -> i32 { x.wrapping_add(87) }
fn n_4_0033(x: i32) -> i32 { n_5_00330(x) + n_5_00331(x) + n_5_00332(x) + n_5_00333(x) }
fn n_3_003(x: i32) -> i32 { n_4_0030(x) + n_4_0031(x) + n_4_0032(x) + n_4_0033(x) }
fn n_2_00(x: i32) -> i32 { n_3_000(x) + n_3_001(x) + n_3_002(x) + n_3_003(x) }
fn n_5_01000(x: i32) -> i32 { x.wrapping_add(91) }
fn n_5_01001(x: i32) -> i32 { x.wrapping_add(92) }
fn n_5_01002(x: i32) -> i32 { x.wrapping_add(93) }
fn n_5_01003(x: i32) -> i32 { x.wrapping_add(94) }
fn n_4_0100(x: i32) -> i32 { n_5_01000(x) + n_5_01001(x) + n_5_01002(x) + n_5_01003(x) }
fn n_5_01010(x: i32) -> i32 { x.wrapping_add(96) }
fn n_5_01011(x: i32) -> i32 { x.wrapping_add(97) }
fn n_5_01012(x: i32) -> i32 { x.wrapping_add(98) }
fn n_5_01013(x: i32) -> i32 { x.wrapping_add(99) }
fn n_4_0101(x: i32) -> i32 { n_5_01010(x) + n_5_01011(x) + n_5_01012(x) + n_5_01013(x) }
fn n_5_01020(x: i32) -> i32 { x.wrapping_add(101) }
fn n_5_01021(x: i32) -> i32 { x.wrapping_add(102) }
fn n_5_01022(x: i32) -> i32 { x.wrapping_add(103) }
fn n_5_01023(x: i32) -> i32 { x.wrapping_add(104) }
fn n_4_0102(x: i32) -> i32 { n_5_01020(x) + n_5_01021(x) + n_5_01022(x) + n_5_01023(x) }
fn n_5_01030(x: i32) -> i32 { x.wrapping_add(106) }
fn n_5_01031(x: i32) -> i32 { x.wrapping_add(107) }
fn n_5_01032(x: i32) -> i32 { x.wrapping_add(108) }
fn n_5_01033(x: i32) -> i32 { x.wrapping_add(109) }
fn n_4_0103(x: i32) -> i32 { n_5_01030(x) + n_5_01031(x) + n_5_01032(x) + n_5_01033(x) }
fn n_3_010(x: i32) -> i32 { n_4_0100(x) + n_4_0101(x) + n_4_0102(x) + n_4_0103(x) }
fn n_5_01100(x: i32) -> i32 { x.wrapping_add(112) }
fn n_5_01101(x: i32) -> i32 { x.wrapping_add(113) }
fn n_5_01102(x: i32) -> i32 { x.wrapping_add(114) }
fn n_5_01103(x: i32) -> i32 { x.wrapping_add(115) }
fn n_4_0110(x: i32) -> i32 { n_5_01100(x) + n_5_01101(x) + n_5_01102(x) + n_5_01103(x) }
fn n_5_01110(x: i32) -> i32 { x.wrapping_add(117) }
fn n_5_01111(x: i32) -> i32 { x.wrapping_add(118) }
fn n_5_01112(x: i32) -> i32 { x.wrapping_add(119) }
fn n_5_01113(x: i32) -> i32 { x.wrapping_add(120) }
fn n_4_0111(x: i32) -> i32 { n_5_01110(x) + n_5_01111(x) + n_5_01112(x) + n_5_01113(x) }
fn n_5_01120(x: i32) -> i32 { x.wrapping_add(122) }
fn n_5_01121(x: i32) -> i32 { x.wrapping_add(123) }
fn n_5_01122(x: i32) -> i32 { x.wrapping_add(124) }
fn n_5_01123(x: i32) -> i32 { x.wrapping_add(125) }
fn n_4_0112(x: i32) -> i32 { n_5_01120(x) + n_5_01121(x) + n_5_01122(x) + n_5_01123(x) }
fn n_5_01130(x: i32) -> i32 { x.wrapping_add(127) }
fn n_5_01131(x: i32) -> i32 { x.wrapping_add(128) }
fn n_5_01132(x: i32) -> i32 { x.wrapping_add(129) }
fn n_5_01133(x: i32) -> i32 { x.wrapping_add(130) }
fn n_4_0113(x: i32) -> i32 { n_5_01130(x) + n_5_01131(x) + n_5_01132(x) + n_5_01133(x) }
fn n_3_011(x: i32) -> i32 { n_4_0110(x) + n_4_0111(x) + n_4_0112(x) + n_4_0113(x) }
fn n_5_01200(x: i32) -> i32 { x.wrapping_add(133) }
fn n_5_01201(x: i32) -> i32 { x.wrapping_add(134) }
fn n_5_01202(x: i32) -> i32 { x.wrapping_add(135) }
fn n_5_01203(x: i32) -> i32 { x.wrapping_add(136) }
fn n_4_0120(x: i32) -> i32 { n_5_01200(x) + n_5_01201(x) + n_5_01202(x) + n_5_01203(x) }
fn n_5_01210(x: i32) -> i32 { x.wrapping_add(138) }
fn n_5_01211(x: i32) -> i32 { x.wrapping_add(139) }
fn n_5_01212(x: i32) -> i32 { x.wrapping_add(140) }
fn n_5_01213(x: i32) -> i32 { x.wrapping_add(141) }
fn n_4_0121(x: i32) -> i32 { n_5_01210(x) + n_5_01211(x) + n_5_01212(x) + n_5_01213(x) }
fn n_5_01220(x: i32) -> i32 { x.wrapping_add(143) }
fn n_5_01221(x: i32) -> i32 { x.wrapping_add(144) }
fn n_5_01222(x: i32) -> i32 { x.wrapping_add(145) }
fn n_5_01223(x: i32) -> i32 { x.wrapping_add(146) }
fn n_4_0122(x: i32) -> i32 { n_5_01220(x) + n_5_01221(x) + n_5_01222(x) + n_5_01223(x) }
fn n_5_01230(x: i32) -> i32 { x.wrapping_add(148) }
fn n_5_01231(x: i32) -> i32 { x.wrapping_add(149) }
fn n_5_01232(x: i32) -> i32 { x.wrapping_add(150) }
fn n_5_01233(x: i32) -> i32 { x.wrapping_add(151) }
fn n_4_0123(x: i32) -> i32 { n_5_01230(x) + n_5_01231(x) + n_5_01232(x) + n_5_01233(x) }
fn n_3_012(x: i32) -> i32 { n_4_0120(x) + n_4_0121(x) + n_4_0122(x) + n_4_0123(x) }
fn n_5_01300(x: i32) -> i32 { x.wrapping_add(154) }
fn n_5_01301(x: i32) -> i32 { x.wrapping_add(155) }
fn n_5_01302(x: i32) -> i32 { x.wrapping_add(156) }
fn n_5_01303(x: i32) -> i32 { x.wrapping_add(157) }
fn n_4_0130(x: i32) -> i32 { n_5_01300(x) + n_5_01301(x) + n_5_01302(x) + n_5_01303(x) }
fn n_5_01310(x: i32) -> i32 { x.wrapping_add(159) }
fn n_5_01311(x: i32) -> i32 { x.wrapping_add(160) }
fn n_5_01312(x: i32) -> i32 { x.wrapping_add(161) }
fn n_5_01313(x: i32) -> i32 { x.wrapping_add(162) }
fn n_4_0131(x: i32) -> i32 { n_5_01310(x) + n_5_01311(x) + n_5_01312(x) + n_5_01313(x) }
fn n_5_01320(x: i32) -> i32 { x.wrapping_add(164) }
fn n_5_01321(x: i32) -> i32 { x.wrapping_add(165) }
fn n_5_01322(x: i32) -> i32 { x.wrapping_add(166) }
fn n_5_01323(x: i32) -> i32 { x.wrapping_add(167) }
fn n_4_0132(x: i32) -> i32 { n_5_01320(x) + n_5_01321(x) + n_5_01322(x) + n_5_01323(x) }
fn n_5_01330(x: i32) -> i32 { x.wrapping_add(169) }
fn n_5_01331(x: i32) -> i32 { x.wrapping_add(170) }
fn n_5_01332(x: i32) -> i32 { x.wrapping_add(171) }
fn n_5_01333(x: i32) -> i32 { x.wrapping_add(172) }
fn n_4_0133(x: i32) -> i32 { n_5_01330(x) + n_5_01331(x) + n_5_01332(x) + n_5_01333(x) }
fn n_3_013(x: i32) -> i32 { n_4_0130(x) + n_4_0131(x) + n_4_0132(x) + n_4_0133(x) }
fn n_2_01(x: i32) -> i32 { n_3_010(x) + n_3_011(x) + n_3_012(x) + n_3_013(x) }
fn n_5_02000(x: i32) -> i32 { x.wrapping_add(176) }
fn n_5_02001(x: i32) -> i32 { x.wrapping_add(177) }
fn n_5_02002(x: i32) -> i32 { x.wrapping_add(178) }
fn n_5_02003(x: i32) -> i32 { x.wrapping_add(179) }
fn n_4_0200(x: i32) -> i32 { n_5_02000(x) + n_5_02001(x) + n_5_02002(x) + n_5_02003(x) }
fn n_5_02010(x: i32) -> i32 { x.wrapping_add(181) }
fn n_5_02011(x: i32) -> i32 { x.wrapping_add(182) }
fn n_5_02012(x: i32) -> i32 { x.wrapping_add(183) }
fn n_5_02013(x: i32) -> i32 { x.wrapping_add(184) }
fn n_4_0201(x: i32) -> i32 { n_5_02010(x) + n_5_02011(x) + n_5_02012(x) + n_5_02013(x) }
fn n_5_02020(x: i32) -> i32 { x.wrapping_add(186) }
fn n_5_02021(x: i32) -> i32 { x.wrapping_add(187) }
fn n_5_02022(x: i32) -> i32 { x.wrapping_add(188) }
fn n_5_02023(x: i32) -> i32 { x.wrapping_add(189) }
fn n_4_0202(x: i32) -> i32 { n_5_02020(x) + n_5_02021(x) + n_5_02022(x) + n_5_02023(x) }
fn n_5_02030(x: i32) -> i32 { x.wrapping_add(191) }
fn n_5_02031(x: i32) -> i32 { x.wrapping_add(192) }
fn n_5_02032(x: i32) -> i32 { x.wrapping_add(193) }
fn n_5_02033(x: i32) -> i32 { x.wrapping_add(194) }
fn n_4_0203(x: i32) -> i32 { n_5_02030(x) + n_5_02031(x) + n_5_02032(x) + n_5_02033(x) }
fn n_3_020(x: i32) -> i32 { n_4_0200(x) + n_4_0201(x) + n_4_0202(x) + n_4_0203(x) }
fn n_5_02100(x: i32) -> i32 { x.wrapping_add(197) }
fn n_5_02101(x: i32) -> i32 { x.wrapping_add(198) }
fn n_5_02102(x: i32) -> i32 { x.wrapping_add(199) }
fn n_5_02103(x: i32) -> i32 { x.wrapping_add(200) }
fn n_4_0210(x: i32) -> i32 { n_5_02100(x) + n_5_02101(x) + n_5_02102(x) + n_5_02103(x) }
fn n_5_02110(x: i32) -> i32 { x.wrapping_add(202) }
fn n_5_02111(x: i32) -> i32 { x.wrapping_add(203) }
fn n_5_02112(x: i32) -> i32 { x.wrapping_add(204) }
fn n_5_02113(x: i32) -> i32 { x.wrapping_add(205) }
fn n_4_0211(x: i32) -> i32 { n_5_02110(x) + n_5_02111(x) + n_5_02112(x) + n_5_02113(x) }
fn n_5_02120(x: i32) -> i32 { x.wrapping_add(207) }
fn n_5_02121(x: i32) -> i32 { x.wrapping_add(208) }
fn n_5_02122(x: i32) -> i32 { x.wrapping_add(209) }
fn n_5_02123(x: i32) -> i32 { x.wrapping_add(210) }
fn n_4_0212(x: i32) -> i32 { n_5_02120(x) + n_5_02121(x) + n_5_02122(x) + n_5_02123(x) }
fn n_5_02130(x: i32) -> i32 { x.wrapping_add(212) }
fn n_5_02131(x: i32) -> i32 { x.wrapping_add(213) }
fn n_5_02132(x: i32) -> i32 { x.wrapping_add(214) }
fn n_5_02133(x: i32) -> i32 { x.wrapping_add(215) }
fn n_4_0213(x: i32) -> i32 { n_5_02130(x) + n_5_02131(x) + n_5_02132(x) + n_5_02133(x) }
fn n_3_021(x: i32) -> i32 { n_4_0210(x) + n_4_0211(x) + n_4_0212(x) + n_4_0213(x) }
fn n_5_02200(x: i32) -> i32 { x.wrapping_add(218) }
fn n_5_02201(x: i32) -> i32 { x.wrapping_add(219) }
fn n_5_02202(x: i32) -> i32 { x.wrapping_add(220) }
fn n_5_02203(x: i32) -> i32 { x.wrapping_add(221) }
fn n_4_0220(x: i32) -> i32 { n_5_02200(x) + n_5_02201(x) + n_5_02202(x) + n_5_02203(x) }
fn n_5_02210(x: i32) -> i32 { x.wrapping_add(223) }
fn n_5_02211(x: i32) -> i32 { x.wrapping_add(224) }
fn n_5_02212(x: i32) -> i32 { x.wrapping_add(225) }
fn n_5_02213(x: i32) -> i32 { x.wrapping_add(226) }
fn n_4_0221(x: i32) -> i32 { n_5_02210(x) + n_5_02211(x) + n_5_02212(x) + n_5_02213(x) }
fn n_5_02220(x: i32) -> i32 { x.wrapping_add(228) }
fn n_5_02221(x: i32) -> i32 { x.wrapping_add(229) }
fn n_5_02222(x: i32) -> i32 { x.wrapping_add(230) }
fn n_5_02223(x: i32) -> i32 { x.wrapping_add(231) }
fn n_4_0222(x: i32) -> i32 { n_5_02220(x) + n_5_02221(x) + n_5_02222(x) + n_5_02223(x) }
fn n_5_02230(x: i32) -> i32 { x.wrapping_add(233) }
fn n_5_02231(x: i32) -> i32 { x.wrapping_add(234) }
fn n_5_02232(x: i32) -> i32 { x.wrapping_add(235) }
fn n_5_02233(x: i32) -> i32 { x.wrapping_add(236) }
fn n_4_0223(x: i32) -> i32 { n_5_02230(x) + n_5_02231(x) + n_5_02232(x) + n_5_02233(x) }
fn n_3_022(x: i32) -> i32 { n_4_0220(x) + n_4_0221(x) + n_4_0222(x) + n_4_0223(x) }
fn n_5_02300(x: i32) -> i32 { x.wrapping_add(239) }
fn n_5_02301(x: i32) -> i32 { x.wrapping_add(240) }
fn n_5_02302(x: i32) -> i32 { x.wrapping_add(241) }
fn n_5_02303(x: i32) -> i32 { x.wrapping_add(242) }
fn n_4_0230(x: i32) -> i32 { n_5_02300(x) + n_5_02301(x) + n_5_02302(x) + n_5_02303(x) }
fn n_5_02310(x: i32) -> i32 { x.wrapping_add(244) }
fn n_5_02311(x: i32) -> i32 { x.wrapping_add(245) }
fn n_5_02312(x: i32) -> i32 { x.wrapping_add(246) }
fn n_5_02313(x: i32) -> i32 { x.wrapping_add(247) }
fn n_4_0231(x: i32) -> i32 { n_5_02310(x) + n_5_02311(x) + n_5_02312(x) + n_5_02313(x) }
fn n_5_02320(x: i32) -> i32 { x.wrapping_add(249) }
fn n_5_02321(x: i32) -> i32 { x.wrapping_add(250) }
fn n_5_02322(x: i32) -> i32 { x.wrapping_add(251) }
fn n_5_02323(x: i32) -> i32 { x.wrapping_add(252) }
fn n_4_0232(x: i32) -> i32 { n_5_02320(x) + n_5_02321(x) + n_5_02322(x) + n_5_02323(x) }
fn n_5_02330(x: i32) -> i32 { x.wrapping_add(254) }
fn n_5_02331(x: i32) -> i32 { x.wrapping_add(255) }
fn n_5_02332(x: i32) -> i32 { x.wrapping_add(256) }
fn n_5_02333(x: i32) -> i32 { x.wrapping_add(257) }
fn n_4_0233(x: i32) -> i32 { n_5_02330(x) + n_5_02331(x) + n_5_02332(x) + n_5_02333(x) }
fn n_3_023(x: i32) -> i32 { n_4_0230(x) + n_4_0231(x) + n_4_0232(x) + n_4_0233(x) }
fn n_2_02(x: i32) -> i32 { n_3_020(x) + n_3_021(x) + n_3_022(x) + n_3_023(x) }
fn n_5_03000(x: i32) -> i32 { x.wrapping_add(261) }
fn n_5_03001(x: i32) -> i32 { x.wrapping_add(262) }
fn n_5_03002(x: i32) -> i32 { x.wrapping_add(263) }
fn n_5_03003(x: i32) -> i32 { x.wrapping_add(264) }
fn n_4_0300(x: i32) -> i32 { n_5_03000(x) + n_5_03001(x) + n_5_03002(x) + n_5_03003(x) }
fn n_5_03010(x: i32) -> i32 { x.wrapping_add(266) }
fn n_5_03011(x: i32) -> i32 { x.wrapping_add(267) }
fn n_5_03012(x: i32) -> i32 { x.wrapping_add(268) }
fn n_5_03013(x: i32) -> i32 { x.wrapping_add(269) }
fn n_4_0301(x: i32) -> i32 { n_5_03010(x) + n_5_03011(x) + n_5_03012(x) + n_5_03013(x) }
fn n_5_03020(x: i32) -> i32 { x.wrapping_add(271) }
fn n_5_03021(x: i32) -> i32 { x.wrapping_add(272) }
fn n_5_03022(x: i32) -> i32 { x.wrapping_add(273) }
fn n_5_03023(x: i32) -> i32 { x.wrapping_add(274) }
fn n_4_0302(x: i32) -> i32 { n_5_03020(x) + n_5_03021(x) + n_5_03022(x) + n_5_03023(x) }
fn n_5_03030(x: i32) -> i32 { x.wrapping_add(276) }
fn n_5_03031(x: i32) -> i32 { x.wrapping_add(277) }
fn n_5_03032(x: i32) -> i32 { x.wrapping_add(278) }
fn n_5_03033(x: i32) -> i32 { x.wrapping_add(279) }
fn n_4_0303(x: i32) -> i32 { n_5_03030(x) + n_5_03031(x) + n_5_03032(x) + n_5_03033(x) }
fn n_3_030(x: i32) -> i32 { n_4_0300(x) + n_4_0301(x) + n_4_0302(x) + n_4_0303(x) }
fn n_5_03100(x: i32) -> i32 { x.wrapping_add(282) }
fn n_5_03101(x: i32) -> i32 { x.wrapping_add(283) }
fn n_5_03102(x: i32) -> i32 { x.wrapping_add(284) }
fn n_5_03103(x: i32) -> i32 { x.wrapping_add(285) }
fn n_4_0310(x: i32) -> i32 { n_5_03100(x) + n_5_03101(x) + n_5_03102(x) + n_5_03103(x) }
fn n_5_03110(x: i32) -> i32 { x.wrapping_add(287) }
fn n_5_03111(x: i32) -> i32 { x.wrapping_add(288) }
fn n_5_03112(x: i32) -> i32 { x.wrapping_add(289) }
fn n_5_03113(x: i32) -> i32 { x.wrapping_add(290) }
fn n_4_0311(x: i32) -> i32 { n_5_03110(x) + n_5_03111(x) + n_5_03112(x) + n_5_03113(x) }
fn n_5_03120(x: i32) -> i32 { x.wrapping_add(292) }
fn n_5_03121(x: i32) -> i32 { x.wrapping_add(293) }
fn n_5_03122(x: i32) -> i32 { x.wrapping_add(294) }
fn n_5_03123(x: i32) -> i32 { x.wrapping_add(295) }
fn n_4_0312(x: i32) -> i32 { n_5_03120(x) + n_5_03121(x) + n_5_03122(x) + n_5_03123(x) }
fn n_5_03130(x: i32) -> i32 { x.wrapping_add(297) }
fn n_5_03131(x: i32) -> i32 { x.wrapping_add(298) }
fn n_5_03132(x: i32) -> i32 { x.wrapping_add(299) }
fn n_5_03133(x: i32) -> i32 { x.wrapping_add(300) }
fn n_4_0313(x: i32) -> i32 { n_5_03130(x) + n_5_03131(x) + n_5_03132(x) + n_5_03133(x) }
fn n_3_031(x: i32) -> i32 { n_4_0310(x) + n_4_0311(x) + n_4_0312(x) + n_4_0313(x) }
fn n_5_03200(x: i32) -> i32 { x.wrapping_add(303) }
fn n_5_03201(x: i32) -> i32 { x.wrapping_add(304) }
fn n_5_03202(x: i32) -> i32 { x.wrapping_add(305) }
fn n_5_03203(x: i32) -> i32 { x.wrapping_add(306) }
fn n_4_0320(x: i32) -> i32 { n_5_03200(x) + n_5_03201(x) + n_5_03202(x) + n_5_03203(x) }
fn n_5_03210(x: i32) -> i32 { x.wrapping_add(308) }
fn n_5_03211(x: i32) -> i32 { x.wrapping_add(309) }
fn n_5_03212(x: i32) -> i32 { x.wrapping_add(310) }
fn n_5_03213(x: i32) -> i32 { x.wrapping_add(311) }
fn n_4_0321(x: i32) -> i32 { n_5_03210(x) + n_5_03211(x) + n_5_03212(x) + n_5_03213(x) }
fn n_5_03220(x: i32) -> i32 { x.wrapping_add(313) }
fn n_5_03221(x: i32) -> i32 { x.wrapping_add(314) }
fn n_5_03222(x: i32) -> i32 { x.wrapping_add(315) }
fn n_5_03223(x: i32) -> i32 { x.wrapping_add(316) }
fn n_4_0322(x: i32) -> i32 { n_5_03220(x) + n_5_03221(x) + n_5_03222(x) + n_5_03223(x) }
fn n_5_03230(x: i32) -> i32 { x.wrapping_add(318) }
fn n_5_03231(x: i32) -> i32 { x.wrapping_add(319) }
fn n_5_03232(x: i32) -> i32 { x.wrapping_add(320) }
fn n_5_03233(x: i32) -> i32 { x.wrapping_add(321) }
fn n_4_0323(x: i32) -> i32 { n_5_03230(x) + n_5_03231(x) + n_5_03232(x) + n_5_03233(x) }
fn n_3_032(x: i32) -> i32 { n_4_0320(x) + n_4_0321(x) + n_4_0322(x) + n_4_0323(x) }
fn n_5_03300(x: i32) -> i32 { x.wrapping_add(324) }
fn n_5_03301(x: i32) -> i32 { x.wrapping_add(325) }
fn n_5_03302(x: i32) -> i32 { x.wrapping_add(326) }
fn n_5_03303(x: i32) -> i32 { x.wrapping_add(327) }
fn n_4_0330(x: i32) -> i32 { n_5_03300(x) + n_5_03301(x) + n_5_03302(x) + n_5_03303(x) }
fn n_5_03310(x: i32) -> i32 { x.wrapping_add(329) }
fn n_5_03311(x: i32) -> i32 { x.wrapping_add(330) }
fn n_5_03312(x: i32) -> i32 { x.wrapping_add(331) }
fn n_5_03313(x: i32) -> i32 { x.wrapping_add(332) }
fn n_4_0331(x: i32) -> i32 { n_5_03310(x) + n_5_03311(x) + n_5_03312(x) + n_5_03313(x) }
fn n_5_03320(x: i32) -> i32 { x.wrapping_add(334) }
fn n_5_03321(x: i32) -> i32 { x.wrapping_add(335) }
fn n_5_03322(x: i32) -> i32 { x.wrapping_add(336) }
fn n_5_03323(x: i32) -> i32 { x.wrapping_add(337) }
fn n_4_0332(x: i32) -> i32 { n_5_03320(x) + n_5_03321(x) + n_5_03322(x) + n_5_03323(x) }
fn n_5_03330(x: i32) -> i32 { x.wrapping_add(339) }
fn n_5_03331(x: i32) -> i32 { x.wrapping_add(340) }
fn n_5_03332(x: i32) -> i32 { x.wrapping_add(341) }
fn n_5_03333(x: i32) -> i32 { x.wrapping_add(342) }
fn n_4_0333(x: i32) -> i32 { n_5_03330(x) + n_5_03331(x) + n_5_03332(x) + n_5_03333(x) }
fn n_3_033(x: i32) -> i32 { n_4_0330(x) + n_4_0331(x) + n_4_0332(x) + n_4_0333(x) }
fn n_2_03(x: i32) -> i32 { n_3_030(x) + n_3_031(x) + n_3_032(x) + n_3_033(x) }
fn n_1_0(x: i32) -> i32 { n_2_00(x) + n_2_01(x) + n_2_02(x) + n_2_03(x) }
fn n_5_10000(x: i32) -> i32 { x.wrapping_add(347) }
fn n_5_10001(x: i32) -> i32 { x.wrapping_add(348) }
fn n_5_10002(x: i32) -> i32 { x.wrapping_add(349) }
fn n_5_10003(x: i32) -> i32 { x.wrapping_add(350) }
fn n_4_1000(x: i32) -> i32 { n_5_10000(x) + n_5_10001(x) + n_5_10002(x) + n_5_10003(x) }
fn n_5_10010(x: i32) -> i32 { x.wrapping_add(352) }
fn n_5_10011(x: i32) -> i32 { x.wrapping_add(353) }
fn n_5_10012(x: i32) -> i32 { x.wrapping_add(354) }
fn n_5_10013(x: i32) -> i32 { x.wrapping_add(355) }
fn n_4_1001(x: i32) -> i32 { n_5_10010(x) + n_5_10011(x) + n_5_10012(x) + n_5_10013(x) }
fn n_5_10020(x: i32) -> i32 { x.wrapping_add(357) }
fn n_5_10021(x: i32) -> i32 { x.wrapping_add(358) }
fn n_5_10022(x: i32) -> i32 { x.wrapping_add(359) }
fn n_5_10023(x: i32) -> i32 { x.wrapping_add(360) }
fn n_4_1002(x: i32) -> i32 { n_5_10020(x) + n_5_10021(x) + n_5_10022(x) + n_5_10023(x) }
fn n_5_10030(x: i32) -> i32 { x.wrapping_add(362) }
fn n_5_10031(x: i32) -> i32 { x.wrapping_add(363) }
fn n_5_10032(x: i32) -> i32 { x.wrapping_add(364) }
fn n_5_10033(x: i32) -> i32 { x.wrapping_add(365) }
fn n_4_1003(x: i32) -> i32 { n_5_10030(x) + n_5_10031(x) + n_5_10032(x) + n_5_10033(x) }
fn n_3_100(x: i32) -> i32 { n_4_1000(x) + n_4_1001(x) + n_4_1002(x) + n_4_1003(x) }
fn n_5_10100(x: i32) -> i32 { x.wrapping_add(368) }
fn n_5_10101(x: i32) -> i32 { x.wrapping_add(369) }
fn n_5_10102(x: i32) -> i32 { x.wrapping_add(370) }
fn n_5_10103(x: i32) -> i32 { x.wrapping_add(371) }
fn n_4_1010(x: i32) -> i32 { n_5_10100(x) + n_5_10101(x) + n_5_10102(x) + n_5_10103(x) }
fn n_5_10110(x: i32) -> i32 { x.wrapping_add(373) }
fn n_5_10111(x: i32) -> i32 { x.wrapping_add(374) }
fn n_5_10112(x: i32) -> i32 { x.wrapping_add(375) }
fn n_5_10113(x: i32) -> i32 { x.wrapping_add(376) }
fn n_4_1011(x: i32) -> i32 { n_5_10110(x) + n_5_10111(x) + n_5_10112(x) + n_5_10113(x) }
fn n_5_10120(x: i32) -> i32 { x.wrapping_add(378) }
fn n_5_10121(x: i32) -> i32 { x.wrapping_add(379) }
fn n_5_10122(x: i32) -> i32 { x.wrapping_add(380) }
fn n_5_10123(x: i32) -> i32 { x.wrapping_add(381) }
fn n_4_1012(x: i32) -> i32 { n_5_10120(x) + n_5_10121(x) + n_5_10122(x) + n_5_10123(x) }
fn n_5_10130(x: i32) -> i32 { x.wrapping_add(383) }
fn n_5_10131(x: i32) -> i32 { x.wrapping_add(384) }
fn n_5_10132(x: i32) -> i32 { x.wrapping_add(385) }
fn n_5_10133(x: i32) -> i32 { x.wrapping_add(386) }
fn n_4_1013(x: i32) -> i32 { n_5_10130(x) + n_5_10131(x) + n_5_10132(x) + n_5_10133(x) }
fn n_3_101(x: i32) -> i32 { n_4_1010(x) + n_4_1011(x) + n_4_1012(x) + n_4_1013(x) }
fn n_5_10200(x: i32) -> i32 { x.wrapping_add(389) }
fn n_5_10201(x: i32) -> i32 { x.wrapping_add(390) }
fn n_5_10202(x: i32) -> i32 { x.wrapping_add(391) }
fn n_5_10203(x: i32) -> i32 { x.wrapping_add(392) }
fn n_4_1020(x: i32) -> i32 { n_5_10200(x) + n_5_10201(x) + n_5_10202(x) + n_5_10203(x) }
fn n_5_10210(x: i32) -> i32 { x.wrapping_add(394) }
fn n_5_10211(x: i32) -> i32 { x.wrapping_add(395) }
fn n_5_10212(x: i32) -> i32 { x.wrapping_add(396) }
fn n_5_10213(x: i32) -> i32 { x.wrapping_add(397) }
fn n_4_1021(x: i32) -> i32 { n_5_10210(x) + n_5_10211(x) + n_5_10212(x) + n_5_10213(x) }
fn n_5_10220(x: i32) -> i32 { x.wrapping_add(399) }
fn n_5_10221(x: i32) -> i32 { x.wrapping_add(400) }
fn n_5_10222(x: i32) -> i32 { x.wrapping_add(401) }
fn n_5_10223(x: i32) -> i32 { x.wrapping_add(402) }
fn n_4_1022(x: i32) -> i32 { n_5_10220(x) + n_5_10221(x) + n_5_10222(x) + n_5_10223(x) }
fn n_5_10230(x: i32) -> i32 { x.wrapping_add(404) }
fn n_5_10231(x: i32) -> i32 { x.wrapping_add(405) }
fn n_5_10232(x: i32) -> i32 { x.wrapping_add(406) }
fn n_5_10233(x: i32) -> i32 { x.wrapping_add(407) }
fn n_4_1023(x: i32) -> i32 { n_5_10230(x) + n_5_10231(x) + n_5_10232(x) + n_5_10233(x) }
fn n_3_102(x: i32) -> i32 { n_4_1020(x) + n_4_1021(x) + n_4_1022(x) + n_4_1023(x) }
fn n_5_10300(x: i32) -> i32 { x.wrapping_add(410) }
fn n_5_10301(x: i32) -> i32 { x.wrapping_add(411) }
fn n_5_10302(x: i32) -> i32 { x.wrapping_add(412) }
fn n_5_10303(x: i32) -> i32 { x.wrapping_add(413) }
fn n_4_1030(x: i32) -> i32 { n_5_10300(x) + n_5_10301(x) + n_5_10302(x) + n_5_10303(x) }
fn n_5_10310(x: i32) -> i32 { x.wrapping_add(415) }
fn n_5_10311(x: i32) -> i32 { x.wrapping_add(416) }
fn n_5_10312(x: i32) -> i32 { x.wrapping_add(417) }
fn n_5_10313(x: i32) -> i32 { x.wrapping_add(418) }
fn n_4_1031(x: i32) -> i32 { n_5_10310(x) + n_5_10311(x) + n_5_10312(x) + n_5_10313(x) }
fn n_5_10320(x: i32) -> i32 { x.wrapping_add(420) }
fn n_5_10321(x: i32) -> i32 { x.wrapping_add(421) }
fn n_5_10322(x: i32) -> i32 { x.wrapping_add(422) }
fn n_5_10323(x: i32) -> i32 { x.wrapping_add(423) }
fn n_4_1032(x: i32) -> i32 { n_5_10320(x) + n_5_10321(x) + n_5_10322(x) + n_5_10323(x) }
fn n_5_10330(x: i32) -> i32 { x.wrapping_add(425) }
fn n_5_10331(x: i32) -> i32 { x.wrapping_add(426) }
fn n_5_10332(x: i32) -> i32 { x.wrapping_add(427) }
fn n_5_10333(x: i32) -> i32 { x.wrapping_add(428) }
fn n_4_1033(x: i32) -> i32 { n_5_10330(x) + n_5_10331(x) + n_5_10332(x) + n_5_10333(x) }
fn n_3_103(x: i32) -> i32 { n_4_1030(x) + n_4_1031(x) + n_4_1032(x) + n_4_1033(x) }
fn n_2_10(x: i32) -> i32 { n_3_100(x) + n_3_101(x) + n_3_102(x) + n_3_103(x) }
fn n_5_11000(x: i32) -> i32 { x.wrapping_add(432) }
fn n_5_11001(x: i32) -> i32 { x.wrapping_add(433) }
fn n_5_11002(x: i32) -> i32 { x.wrapping_add(434) }
fn n_5_11003(x: i32) -> i32 { x.wrapping_add(435) }
fn n_4_1100(x: i32) -> i32 { n_5_11000(x) + n_5_11001(x) + n_5_11002(x) + n_5_11003(x) }
fn n_5_11010(x: i32) -> i32 { x.wrapping_add(437) }
fn n_5_11011(x: i32) -> i32 { x.wrapping_add(438) }
fn n_5_11012(x: i32) -> i32 { x.wrapping_add(439) }
fn n_5_11013(x: i32) -> i32 { x.wrapping_add(440) }
fn n_4_1101(x: i32) -> i32 { n_5_11010(x) + n_5_11011(x) + n_5_11012(x) + n_5_11013(x) }
fn n_5_11020(x: i32) -> i32 { x.wrapping_add(442) }
fn n_5_11021(x: i32) -> i32 { x.wrapping_add(443) }
fn n_5_11022(x: i32) -> i32 { x.wrapping_add(444) }
fn n_5_11023(x: i32) -> i32 { x.wrapping_add(445) }
fn n_4_1102(x: i32) -> i32 { n_5_11020(x) + n_5_11021(x) + n_5_11022(x) + n_5_11023(x) }
fn n_5_11030(x: i32) -> i32 { x.wrapping_add(447) }
fn n_5_11031(x: i32) -> i32 { x.wrapping_add(448) }
fn n_5_11032(x: i32) -> i32 { x.wrapping_add(449) }
fn n_5_11033(x: i32) -> i32 { x.wrapping_add(450) }
fn n_4_1103(x: i32) -> i32 { n_5_11030(x) + n_5_11031(x) + n_5_11032(x) + n_5_11033(x) }
fn n_3_110(x: i32) -> i32 { n_4_1100(x) + n_4_1101(x) + n_4_1102(x) + n_4_1103(x) }
fn n_5_11100(x: i32) -> i32 { x.wrapping_add(453) }
fn n_5_11101(x: i32) -> i32 { x.wrapping_add(454) }
fn n_5_11102(x: i32) -> i32 { x.wrapping_add(455) }
fn n_5_11103(x: i32) -> i32 { x.wrapping_add(456) }
fn n_4_1110(x: i32) -> i32 { n_5_11100(x) + n_5_11101(x) + n_5_11102(x) + n_5_11103(x) }
fn n_5_11110(x: i32) -> i32 { x.wrapping_add(458) }
fn n_5_11111(x: i32) -> i32 { x.wrapping_add(459) }
fn n_5_11112(x: i32) -> i32 { x.wrapping_add(460) }
fn n_5_11113(x: i32) -> i32 { x.wrapping_add(461) }
fn n_4_1111(x: i32) -> i32 { n_5_11110(x) + n_5_11111(x) + n_5_11112(x) + n_5_11113(x) }
fn n_5_11120(x: i32) -> i32 { x.wrapping_add(463) }
fn n_5_11121(x: i32) -> i32 { x.wrapping_add(464) }
fn n_5_11122(x: i32) -> i32 { x.wrapping_add(465) }
fn n_5_11123(x: i32) -> i32 { x.wrapping_add(466) }
fn n_4_1112(x: i32) -> i32 { n_5_11120(x) + n_5_11121(x) + n_5_11122(x) + n_5_11123(x) }
fn n_5_11130(x: i32) -> i32 { x.wrapping_add(468) }
fn n_5_11131(x: i32) -> i32 { x.wrapping_add(469) }
fn n_5_11132(x: i32) -> i32 { x.wrapping_add(470) }
fn n_5_11133(x: i32) -> i32 { x.wrapping_add(471) }
fn n_4_1113(x: i32) -> i32 { n_5_11130(x) + n_5_11131(x) + n_5_11132(x) + n_5_11133(x) }
fn n_3_111(x: i32) -> i32 { n_4_1110(x) + n_4_1111(x) + n_4_1112(x) + n_4_1113(x) }
fn n_5_11200(x: i32) -> i32 { x.wrapping_add(474) }
fn n_5_11201(x: i32) -> i32 { x.wrapping_add(475) }
fn n_5_11202(x: i32) -> i32 { x.wrapping_add(476) }
fn n_5_11203(x: i32) -> i32 { x.wrapping_add(477) }
fn n_4_1120(x: i32) -> i32 { n_5_11200(x) + n_5_11201(x) + n_5_11202(x) + n_5_11203(x) }
fn n_5_11210(x: i32) -> i32 { x.wrapping_add(479) }
fn n_5_11211(x: i32) -> i32 { x.wrapping_add(480) }
fn n_5_11212(x: i32) -> i32 { x.wrapping_add(481) }
fn n_5_11213(x: i32) -> i32 { x.wrapping_add(482) }
fn n_4_1121(x: i32) -> i32 { n_5_11210(x) + n_5_11211(x) + n_5_11212(x) + n_5_11213(x) }
fn n_5_11220(x: i32) -> i32 { x.wrapping_add(484) }
fn n_5_11221(x: i32) -> i32 { x.wrapping_add(485) }
fn n_5_11222(x: i32) -> i32 { x.wrapping_add(486) }
fn n_5_11223(x: i32) -> i32 { x.wrapping_add(487) }
fn n_4_1122(x: i32) -> i32 { n_5_11220(x) + n_5_11221(x) + n_5_11222(x) + n_5_11223(x) }
fn n_5_11230(x: i32) -> i32 { x.wrapping_add(489) }
fn n_5_11231(x: i32) -> i32 { x.wrapping_add(490) }
fn n_5_11232(x: i32) -> i32 { x.wrapping_add(491) }
fn n_5_11233(x: i32) -> i32 { x.wrapping_add(492) }
fn n_4_1123(x: i32) -> i32 { n_5_11230(x) + n_5_11231(x) + n_5_11232(x) + n_5_11233(x) }
fn n_3_112(x: i32) -> i32 { n_4_1120(x) + n_4_1121(x) + n_4_1122(x) + n_4_1123(x) }
fn n_5_11300(x: i32) -> i32 { x.wrapping_add(495) }
fn n_5_11301(x: i32) -> i32 { x.wrapping_add(496) }
fn n_5_11302(x: i32) -> i32 { x.wrapping_add(497) }
fn n_5_11303(x: i32) -> i32 { x.wrapping_add(498) }
fn n_4_1130(x: i32) -> i32 { n_5_11300(x) + n_5_11301(x) + n_5_11302(x) + n_5_11303(x) }
fn n_5_11310(x: i32) -> i32 { x.wrapping_add(500) }
fn n_5_11311(x: i32) -> i32 { x.wrapping_add(501) }
fn n_5_11312(x: i32) -> i32 { x.wrapping_add(502) }
fn n_5_11313(x: i32) -> i32 { x.wrapping_add(503) }
fn n_4_1131(x: i32) -> i32 { n_5_11310(x) + n_5_11311(x) + n_5_11312(x) + n_5_11313(x) }
fn n_5_11320(x: i32) -> i32 { x.wrapping_add(505) }
fn n_5_11321(x: i32) -> i32 { x.wrapping_add(506) }
fn n_5_11322(x: i32) -> i32 { x.wrapping_add(507) }
fn n_5_11323(x: i32) -> i32 { x.wrapping_add(508) }
fn n_4_1132(x: i32) -> i32 { n_5_11320(x) + n_5_11321(x) + n_5_11322(x) + n_5_11323(x) }
fn n_5_11330(x: i32) -> i32 { x.wrapping_add(510) }
fn n_5_11331(x: i32) -> i32 { x.wrapping_add(511) }
fn n_5_11332(x: i32) -> i32 { x.wrapping_add(512) }
fn n_5_11333(x: i32) -> i32 { x.wrapping_add(513) }
fn n_4_1133(x: i32) -> i32 { n_5_11330(x) + n_5_11331(x) + n_5_11332(x) + n_5_11333(x) }
fn n_3_113(x: i32) -> i32 { n_4_1130(x) + n_4_1131(x) + n_4_1132(x) + n_4_1133(x) }
fn n_2_11(x: i32) -> i32 { n_3_110(x) + n_3_111(x) + n_3_112(x) + n_3_113(x) }
fn n_5_12000(x: i32) -> i32 { x.wrapping_add(517) }
fn n_5_12001(x: i32) -> i32 { x.wrapping_add(518) }
fn n_5_12002(x: i32) -> i32 { x.wrapping_add(519) }
fn n_5_12003(x: i32) -> i32 { x.wrapping_add(520) }
fn n_4_1200(x: i32) -> i32 { n_5_12000(x) + n_5_12001(x) + n_5_12002(x) + n_5_12003(x) }
fn n_5_12010(x: i32) -> i32 { x.wrapping_add(522) }
fn n_5_12011(x: i32) -> i32 { x.wrapping_add(523) }
fn n_5_12012(x: i32) -> i32 { x.wrapping_add(524) }
fn n_5_12013(x: i32) -> i32 { x.wrapping_add(525) }
fn n_4_1201(x: i32) -> i32 { n_5_12010(x) + n_5_12011(x) + n_5_12012(x) + n_5_12013(x) }
fn n_5_12020(x: i32) -> i32 { x.wrapping_add(527) }
fn n_5_12021(x: i32) -> i32 { x.wrapping_add(528) }
fn n_5_12022(x: i32) -> i32 { x.wrapping_add(529) }
fn n_5_12023(x: i32) -> i32 { x.wrapping_add(530) }
fn n_4_1202(x: i32) -> i32 { n_5_12020(x) + n_5_12021(x) + n_5_12022(x) + n_5_12023(x) }
fn n_5_12030(x: i32) -> i32 { x.wrapping_add(532) }
fn n_5_12031(x: i32) -> i32 { x.wrapping_add(533) }
fn n_5_12032(x: i32) -> i32 { x.wrapping_add(534) }
fn n_5_12033(x: i32) -> i32 { x.wrapping_add(535) }
fn n_4_1203(x: i32) -> i32 { n_5_12030(x) + n_5_12031(x) + n_5_12032(x) + n_5_12033(x) }
fn n_3_120(x: i32) -> i32 { n_4_1200(x) + n_4_1201(x) + n_4_1202(x) + n_4_1203(x) }
fn n_5_12100(x: i32) -> i32 { x.wrapping_add(538) }
fn n_5_12101(x: i32) -> i32 { x.wrapping_add(539) }
fn n_5_12102(x: i32) -> i32 { x.wrapping_add(540) }
fn n_5_12103(x: i32) -> i32 { x.wrapping_add(541) }
fn n_4_1210(x: i32) -> i32 { n_5_12100(x) + n_5_12101(x) + n_5_12102(x) + n_5_12103(x) }
fn n_5_12110(x: i32) -> i32 { x.wrapping_add(543) }
fn n_5_12111(x: i32) -> i32 { x.wrapping_add(544) }
fn n_5_12112(x: i32) -> i32 { x.wrapping_add(545) }
fn n_5_12113(x: i32) -> i32 { x.wrapping_add(546) }
fn n_4_1211(x: i32) -> i32 { n_5_12110(x) + n_5_12111(x) + n_5_12112(x) + n_5_12113(x) }
fn n_5_12120(x: i32) -> i32 { x.wrapping_add(548) }
fn n_5_12121(x: i32) -> i32 { x.wrapping_add(549) }
fn n_5_12122(x: i32) -> i32 { x.wrapping_add(550) }
fn n_5_12123(x: i32) -> i32 { x.wrapping_add(551) }
fn n_4_1212(x: i32) -> i32 { n_5_12120(x) + n_5_12121(x) + n_5_12122(x) + n_5_12123(x) }
fn n_5_12130(x: i32) -> i32 { x.wrapping_add(553) }
fn n_5_12131(x: i32) -> i32 { x.wrapping_add(554) }
fn n_5_12132(x: i32) -> i32 { x.wrapping_add(555) }
fn n_5_12133(x: i32) -> i32 { x.wrapping_add(556) }
fn n_4_1213(x: i32) -> i32 { n_5_12130(x) + n_5_12131(x) + n_5_12132(x) + n_5_12133(x) }
fn n_3_121(x: i32) -> i32 { n_4_1210(x) + n_4_1211(x) + n_4_1212(x) + n_4_1213(x) }
fn n_5_12200(x: i32) -> i32 { x.wrapping_add(559) }
fn n_5_12201(x: i32) -> i32 { x.wrapping_add(560) }
fn n_5_12202(x: i32) -> i32 { x.wrapping_add(561) }
fn n_5_12203(x: i32) -> i32 { x.wrapping_add(562) }
fn n_4_1220(x: i32) -> i32 { n_5_12200(x) + n_5_12201(x) + n_5_12202(x) + n_5_12203(x) }
fn n_5_12210(x: i32) -> i32 { x.wrapping_add(564) }
fn n_5_12211(x: i32) -> i32 { x.wrapping_add(565) }
fn n_5_12212(x: i32) -> i32 { x.wrapping_add(566) }
fn n_5_12213(x: i32) -> i32 { x.wrapping_add(567) }
fn n_4_1221(x: i32) -> i32 { n_5_12210(x) + n_5_12211(x) + n_5_12212(x) + n_5_12213(x) }
fn n_5_12220(x: i32) -> i32 { x.wrapping_add(569) }
fn n_5_12221(x: i32) -> i32 { x.wrapping_add(570) }
fn n_5_12222(x: i32) -> i32 { x.wrapping_add(571) }
fn n_5_12223(x: i32) -> i32 { x.wrapping_add(572) }
fn n_4_1222(x: i32) -> i32 { n_5_12220(x) + n_5_12221(x) + n_5_12222(x) + n_5_12223(x) }
fn n_5_12230(x: i32) -> i32 { x.wrapping_add(574) }
fn n_5_12231(x: i32) -> i32 { x.wrapping_add(575) }
fn n_5_12232(x: i32) -> i32 { x.wrapping_add(576) }
fn n_5_12233(x: i32) -> i32 { x.wrapping_add(577) }
fn n_4_1223(x: i32) -> i32 { n_5_12230(x) + n_5_12231(x) + n_5_12232(x) + n_5_12233(x) }
fn n_3_122(x: i32) -> i32 { n_4_1220(x) + n_4_1221(x) + n_4_1222(x) + n_4_1223(x) }
fn n_5_12300(x: i32) -> i32 { x.wrapping_add(580) }
fn n_5_12301(x: i32) -> i32 { x.wrapping_add(581) }
fn n_5_12302(x: i32) -> i32 { x.wrapping_add(582) }
fn n_5_12303(x: i32) -> i32 { x.wrapping_add(583) }
fn n_4_1230(x: i32) -> i32 { n_5_12300(x) + n_5_12301(x) + n_5_12302(x) + n_5_12303(x) }
fn n_5_12310(x: i32) -> i32 { x.wrapping_add(585) }
fn n_5_12311(x: i32) -> i32 { x.wrapping_add(586) }
fn n_5_12312(x: i32) -> i32 { x.wrapping_add(587) }
fn n_5_12313(x: i32) -> i32 { x.wrapping_add(588) }
fn n_4_1231(x: i32) -> i32 { n_5_12310(x) + n_5_12311(x) + n_5_12312(x) + n_5_12313(x) }
fn n_5_12320(x: i32) -> i32 { x.wrapping_add(590) }
fn n_5_12321(x: i32) -> i32 { x.wrapping_add(591) }
fn n_5_12322(x: i32) -> i32 { x.wrapping_add(592) }
fn n_5_12323(x: i32) -> i32 { x.wrapping_add(593) }
fn n_4_1232(x: i32) -> i32 { n_5_12320(x) + n_5_12321(x) + n_5_12322(x) + n_5_12323(x) }
fn n_5_12330(x: i32) -> i32 { x.wrapping_add(595) }
fn n_5_12331(x: i32) -> i32 { x.wrapping_add(596) }
fn n_5_12332(x: i32) -> i32 { x.wrapping_add(597) }
fn n_5_12333(x: i32) -> i32 { x.wrapping_add(598) }
fn n_4_1233(x: i32) -> i32 { n_5_12330(x) + n_5_12331(x) + n_5_12332(x) + n_5_12333(x) }
fn n_3_123(x: i32) -> i32 { n_4_1230(x) + n_4_1231(x) + n_4_1232(x) + n_4_1233(x) }
fn n_2_12(x: i32) -> i32 { n_3_120(x) + n_3_121(x) + n_3_122(x) + n_3_123(x) }
fn n_5_13000(x: i32) -> i32 { x.wrapping_add(602) }
fn n_5_13001(x: i32) -> i32 { x.wrapping_add(603) }
fn n_5_13002(x: i32) -> i32 { x.wrapping_add(604) }
fn n_5_13003(x: i32) -> i32 { x.wrapping_add(605) }
fn n_4_1300(x: i32) -> i32 { n_5_13000(x) + n_5_13001(x) + n_5_13002(x) + n_5_13003(x) }
fn n_5_13010(x: i32) -> i32 { x.wrapping_add(607) }
fn n_5_13011(x: i32) -> i32 { x.wrapping_add(608) }
fn n_5_13012(x: i32) -> i32 { x.wrapping_add(609) }
fn n_5_13013(x: i32) -> i32 { x.wrapping_add(610) }
fn n_4_1301(x: i32) -> i32 { n_5_13010(x) + n_5_13011(x) + n_5_13012(x) + n_5_13013(x) }
fn n_5_13020(x: i32) -> i32 { x.wrapping_add(612) }
fn n_5_13021(x: i32) -> i32 { x.wrapping_add(613) }
fn n_5_13022(x: i32) -> i32 { x.wrapping_add(614) }
fn n_5_13023(x: i32) -> i32 { x.wrapping_add(615) }
fn n_4_1302(x: i32) -> i32 { n_5_13020(x) + n_5_13021(x) + n_5_13022(x) + n_5_13023(x) }
fn n_5_13030(x: i32) -> i32 { x.wrapping_add(617) }
fn n_5_13031(x: i32) -> i32 { x.wrapping_add(618) }
fn n_5_13032(x: i32) -> i32 { x.wrapping_add(619) }
fn n_5_13033(x: i32) -> i32 { x.wrapping_add(620) }
fn n_4_1303(x: i32) -> i32 { n_5_13030(x) + n_5_13031(x) + n_5_13032(x) + n_5_13033(x) }
fn n_3_130(x: i32) -> i32 { n_4_1300(x) + n_4_1301(x) + n_4_1302(x) + n_4_1303(x) }
fn n_5_13100(x: i32) -> i32 { x.wrapping_add(623) }
fn n_5_13101(x: i32) -> i32 { x.wrapping_add(624) }
fn n_5_13102(x: i32) -> i32 { x.wrapping_add(625) }
fn n_5_13103(x: i32) -> i32 { x.wrapping_add(626) }
fn n_4_1310(x: i32) -> i32 { n_5_13100(x) + n_5_13101(x) + n_5_13102(x) + n_5_13103(x) }
fn n_5_13110(x: i32) -> i32 { x.wrapping_add(628) }
fn n_5_13111(x: i32) -> i32 { x.wrapping_add(629) }
fn n_5_13112(x: i32) -> i32 { x.wrapping_add(630) }
fn n_5_13113(x: i32) -> i32 { x.wrapping_add(631) }
fn n_4_1311(x: i32) -> i32 { n_5_13110(x) + n_5_13111(x) + n_5_13112(x) + n_5_13113(x) }
fn n_5_13120(x: i32) -> i32 { x.wrapping_add(633) }
fn n_5_13121(x: i32) -> i32 { x.wrapping_add(634) }
fn n_5_13122(x: i32) -> i32 { x.wrapping_add(635) }
fn n_5_13123(x: i32) -> i32 { x.wrapping_add(636) }
fn n_4_1312(x: i32) -> i32 { n_5_13120(x) + n_5_13121(x) + n_5_13122(x) + n_5_13123(x) }
fn n_5_13130(x: i32) -> i32 { x.wrapping_add(638) }
fn n_5_13131(x: i32) -> i32 { x.wrapping_add(639) }
fn n_5_13132(x: i32) -> i32 { x.wrapping_add(640) }
fn n_5_13133(x: i32) -> i32 { x.wrapping_add(641) }
fn n_4_1313(x: i32) -> i32 { n_5_13130(x) + n_5_13131(x) + n_5_13132(x) + n_5_13133(x) }
fn n_3_131(x: i32) -> i32 { n_4_1310(x) + n_4_1311(x) + n_4_1312(x) + n_4_1313(x) }
fn n_5_13200(x: i32) -> i32 { x.wrapping_add(644) }
fn n_5_13201(x: i32) -> i32 { x.wrapping_add(645) }
fn n_5_13202(x: i32) -> i32 { x.wrapping_add(646) }
fn n_5_13203(x: i32) -> i32 { x.wrapping_add(647) }
fn n_4_1320(x: i32) -> i32 { n_5_13200(x) + n_5_13201(x) + n_5_13202(x) + n_5_13203(x) }
fn n_5_13210(x: i32) -> i32 { x.wrapping_add(649) }
fn n_5_13211(x: i32) -> i32 { x.wrapping_add(650) }
fn n_5_13212(x: i32) -> i32 { x.wrapping_add(651) }
fn n_5_13213(x: i32) -> i32 { x.wrapping_add(652) }
fn n_4_1321(x: i32) -> i32 { n_5_13210(x) + n_5_13211(x) + n_5_13212(x) + n_5_13213(x) }
fn n_5_13220(x: i32) -> i32 { x.wrapping_add(654) }
fn n_5_13221(x: i32) -> i32 { x.wrapping_add(655) }
fn n_5_13222(x: i32) -> i32 { x.wrapping_add(656) }
fn n_5_13223(x: i32) -> i32 { x.wrapping_add(657) }
fn n_4_1322(x: i32) -> i32 { n_5_13220(x) + n_5_13221(x) + n_5_13222(x) + n_5_13223(x) }
fn n_5_13230(x: i32) -> i32 { x.wrapping_add(659) }
fn n_5_13231(x: i32) -> i32 { x.wrapping_add(660) }
fn n_5_13232(x: i32) -> i32 { x.wrapping_add(661) }
fn n_5_13233(x: i32) -> i32 { x.wrapping_add(662) }
fn n_4_1323(x: i32) -> i32 { n_5_13230(x) + n_5_13231(x) + n_5_13232(x) + n_5_13233(x) }
fn n_3_132(x: i32) -> i32 { n_4_1320(x) + n_4_1321(x) + n_4_1322(x) + n_4_1323(x) }
fn n_5_13300(x: i32) -> i32 { x.wrapping_add(665) }
fn n_5_13301(x: i32) -> i32 { x.wrapping_add(666) }
fn n_5_13302(x: i32) -> i32 { x.wrapping_add(667) }
fn n_5_13303(x: i32) -> i32 { x.wrapping_add(668) }
fn n_4_1330(x: i32) -> i32 { n_5_13300(x) + n_5_13301(x) + n_5_13302(x) + n_5_13303(x) }
fn n_5_13310(x: i32) -> i32 { x.wrapping_add(670) }
fn n_5_13311(x: i32) -> i32 { x.wrapping_add(671) }
fn n_5_13312(x: i32) -> i32 { x.wrapping_add(672) }
fn n_5_13313(x: i32) -> i32 { x.wrapping_add(673) }
fn n_4_1331(x: i32) -> i32 { n_5_13310(x) + n_5_13311(x) + n_5_13312(x) + n_5_13313(x) }
fn n_5_13320(x: i32) -> i32 { x.wrapping_add(675) }
fn n_5_13321(x: i32) -> i32 { x.wrapping_add(676) }
fn n_5_13322(x: i32) -> i32 { x.wrapping_add(677) }
fn n_5_13323(x: i32) -> i32 { x.wrapping_add(678) }
fn n_4_1332(x: i32) -> i32 { n_5_13320(x) + n_5_13321(x) + n_5_13322(x) + n_5_13323(x) }
fn n_5_13330(x: i32) -> i32 { x.wrapping_add(680) }
fn n_5_13331(x: i32) -> i32 { x.wrapping_add(681) }
fn n_5_13332(x: i32) -> i32 { x.wrapping_add(682) }
fn n_5_13333(x: i32) -> i32 { x.wrapping_add(683) }
fn n_4_1333(x: i32) -> i32 { n_5_13330(x) + n_5_13331(x) + n_5_13332(x) + n_5_13333(x) }
fn n_3_133(x: i32) -> i32 { n_4_1330(x) + n_4_1331(x) + n_4_1332(x) + n_4_1333(x) }
fn n_2_13(x: i32) -> i32 { n_3_130(x) + n_3_131(x) + n_3_132(x) + n_3_133(x) }
fn n_1_1(x: i32) -> i32 { n_2_10(x) + n_2_11(x) + n_2_12(x) + n_2_13(x) }
fn n_5_20000(x: i32) -> i32 { x.wrapping_add(688) }
fn n_5_20001(x: i32) -> i32 { x.wrapping_add(689) }
fn n_5_20002(x: i32) -> i32 { x.wrapping_add(690) }
fn n_5_20003(x: i32) -> i32 { x.wrapping_add(691) }
fn n_4_2000(x: i32) -> i32 { n_5_20000(x) + n_5_20001(x) + n_5_20002(x) + n_5_20003(x) }
fn n_5_20010(x: i32) -> i32 { x.wrapping_add(693) }
fn n_5_20011(x: i32) -> i32 { x.wrapping_add(694) }
fn n_5_20012(x: i32) -> i32 { x.wrapping_add(695) }
fn n_5_20013(x: i32) -> i32 { x.wrapping_add(696) }
fn n_4_2001(x: i32) -> i32 { n_5_20010(x) + n_5_20011(x) + n_5_20012(x) + n_5_20013(x) }
fn n_5_20020(x: i32) -> i32 { x.wrapping_add(698) }
fn n_5_20021(x: i32) -> i32 { x.wrapping_add(699) }
fn n_5_20022(x: i32) -> i32 { x.wrapping_add(700) }
fn n_5_20023(x: i32) -> i32 { x.wrapping_add(701) }
fn n_4_2002(x: i32) -> i32 { n_5_20020(x) + n_5_20021(x) + n_5_20022(x) + n_5_20023(x) }
fn n_5_20030(x: i32) -> i32 { x.wrapping_add(703) }
fn n_5_20031(x: i32) -> i32 { x.wrapping_add(704) }
fn n_5_20032(x: i32) -> i32 { x.wrapping_add(705) }
fn n_5_20033(x: i32) -> i32 { x.wrapping_add(706) }
fn n_4_2003(x: i32) -> i32 { n_5_20030(x) + n_5_20031(x) + n_5_20032(x) + n_5_20033(x) }
fn n_3_200(x: i32) -> i32 { n_4_2000(x) + n_4_2001(x) + n_4_2002(x) + n_4_2003(x) }
fn n_5_20100(x: i32) -> i32 { x.wrapping_add(709) }
fn n_5_20101(x: i32) -> i32 { x.wrapping_add(710) }
fn n_5_20102(x: i32) -> i32 { x.wrapping_add(711) }
fn n_5_20103(x: i32) -> i32 { x.wrapping_add(712) }
fn n_4_2010(x: i32) -> i32 { n_5_20100(x) + n_5_20101(x) + n_5_20102(x) + n_5_20103(x) }
fn n_5_20110(x: i32) -> i32 { x.wrapping_add(714) }
fn n_5_20111(x: i32) -> i32 { x.wrapping_add(715) }
fn n_5_20112(x: i32) -> i32 { x.wrapping_add(716) }
fn n_5_20113(x: i32) -> i32 { x.wrapping_add(717) }
fn n_4_2011(x: i32) -> i32 { n_5_20110(x) + n_5_20111(x) + n_5_20112(x) + n_5_20113(x) }
fn n_5_20120(x: i32) -> i32 { x.wrapping_add(719) }
fn n_5_20121(x: i32) -> i32 { x.wrapping_add(720) }
fn n_5_20122(x: i32) -> i32 { x.wrapping_add(721) }
fn n_5_20123(x: i32) -> i32 { x.wrapping_add(722) }
fn n_4_2012(x: i32) -> i32 { n_5_20120(x) + n_5_20121(x) + n_5_20122(x) + n_5_20123(x) }
fn n_5_20130(x: i32) -> i32 { x.wrapping_add(724) }
fn n_5_20131(x: i32) -> i32 { x.wrapping_add(725) }
fn n_5_20132(x: i32) -> i32 { x.wrapping_add(726) }
fn n_5_20133(x: i32) -> i32 { x.wrapping_add(727) }
fn n_4_2013(x: i32) -> i32 { n_5_20130(x) + n_5_20131(x) + n_5_20132(x) + n_5_20133(x) }
fn n_3_201(x: i32) -> i32 { n_4_2010(x) + n_4_2011(x) + n_4_2012(x) + n_4_2013(x) }
fn n_5_20200(x: i32) -> i32 { x.wrapping_add(730) }
fn n_5_20201(x: i32) -> i32 { x.wrapping_add(731) }
fn n_5_20202(x: i32) -> i32 { x.wrapping_add(732) }
fn n_5_20203(x: i32) -> i32 { x.wrapping_add(733) }
fn n_4_2020(x: i32) -> i32 { n_5_20200(x) + n_5_20201(x) + n_5_20202(x) + n_5_20203(x) }
fn n_5_20210(x: i32) -> i32 { x.wrapping_add(735) }
fn n_5_20211(x: i32) -> i32 { x.wrapping_add(736) }
fn n_5_20212(x: i32) -> i32 { x.wrapping_add(737) }
fn n_5_20213(x: i32) -> i32 { x.wrapping_add(738) }
fn n_4_2021(x: i32) -> i32 { n_5_20210(x) + n_5_20211(x) + n_5_20212(x) + n_5_20213(x) }
fn n_5_20220(x: i32) -> i32 { x.wrapping_add(740) }
fn n_5_20221(x: i32) -> i32 { x.wrapping_add(741) }
fn n_5_20222(x: i32) -> i32 { x.wrapping_add(742) }
fn n_5_20223(x: i32) -> i32 { x.wrapping_add(743) }
fn n_4_2022(x: i32) -> i32 { n_5_20220(x) + n_5_20221(x) + n_5_20222(x) + n_5_20223(x) }
fn n_5_20230(x: i32) -> i32 { x.wrapping_add(745) }
fn n_5_20231(x: i32) -> i32 { x.wrapping_add(746) }
fn n_5_20232(x: i32) -> i32 { x.wrapping_add(747) }
fn n_5_20233(x: i32) -> i32 { x.wrapping_add(748) }
fn n_4_2023(x: i32) -> i32 { n_5_20230(x) + n_5_20231(x) + n_5_20232(x) + n_5_20233(x) }
fn n_3_202(x: i32) -> i32 { n_4_2020(x) + n_4_2021(x) + n_4_2022(x) + n_4_2023(x) }
fn n_5_20300(x: i32) -> i32 { x.wrapping_add(751) }
fn n_5_20301(x: i32) -> i32 { x.wrapping_add(752) }
fn n_5_20302(x: i32) -> i32 { x.wrapping_add(753) }
fn n_5_20303(x: i32) -> i32 { x.wrapping_add(754) }
fn n_4_2030(x: i32) -> i32 { n_5_20300(x) + n_5_20301(x) + n_5_20302(x) + n_5_20303(x) }
fn n_5_20310(x: i32) -> i32 { x.wrapping_add(756) }
fn n_5_20311(x: i32) -> i32 { x.wrapping_add(757) }
fn n_5_20312(x: i32) -> i32 { x.wrapping_add(758) }
fn n_5_20313(x: i32) -> i32 { x.wrapping_add(759) }
fn n_4_2031(x: i32) -> i32 { n_5_20310(x) + n_5_20311(x) + n_5_20312(x) + n_5_20313(x) }
fn n_5_20320(x: i32) -> i32 { x.wrapping_add(761) }
fn n_5_20321(x: i32) -> i32 { x.wrapping_add(762) }
fn n_5_20322(x: i32) -> i32 { x.wrapping_add(763) }
fn n_5_20323(x: i32) -> i32 { x.wrapping_add(764) }
fn n_4_2032(x: i32) -> i32 { n_5_20320(x) + n_5_20321(x) + n_5_20322(x) + n_5_20323(x) }
fn n_5_20330(x: i32) -> i32 { x.wrapping_add(766) }
fn n_5_20331(x: i32) -> i32 { x.wrapping_add(767) }
fn n_5_20332(x: i32) -> i32 { x.wrapping_add(768) }
fn n_5_20333(x: i32) -> i32 { x.wrapping_add(769) }
fn n_4_2033(x: i32) -> i32 { n_5_20330(x) + n_5_20331(x) + n_5_20332(x) + n_5_20333(x) }
fn n_3_203(x: i32) -> i32 { n_4_2030(x) + n_4_2031(x) + n_4_2032(x) + n_4_2033(x) }
fn n_2_20(x: i32) -> i32 { n_3_200(x) + n_3_201(x) + n_3_202(x) + n_3_203(x) }
fn n_5_21000(x: i32) -> i32 { x.wrapping_add(773) }
fn n_5_21001(x: i32) -> i32 { x.wrapping_add(774) }
fn n_5_21002(x: i32) -> i32 { x.wrapping_add(775) }
fn n_5_21003(x: i32) -> i32 { x.wrapping_add(776) }
fn n_4_2100(x: i32) -> i32 { n_5_21000(x) + n_5_21001(x) + n_5_21002(x) + n_5_21003(x) }
fn n_5_21010(x: i32) -> i32 { x.wrapping_add(778) }
fn n_5_21011(x: i32) -> i32 { x.wrapping_add(779) }
fn n_5_21012(x: i32) -> i32 { x.wrapping_add(780) }
fn n_5_21013(x: i32) -> i32 { x.wrapping_add(781) }
fn n_4_2101(x: i32) -> i32 { n_5_21010(x) + n_5_21011(x) + n_5_21012(x) + n_5_21013(x) }
fn n_5_21020(x: i32) -> i32 { x.wrapping_add(783) }
fn n_5_21021(x: i32) -> i32 { x.wrapping_add(784) }
fn n_5_21022(x: i32) -> i32 { x.wrapping_add(785) }
fn n_5_21023(x: i32) -> i32 { x.wrapping_add(786) }
fn n_4_2102(x: i32) -> i32 { n_5_21020(x) + n_5_21021(x) + n_5_21022(x) + n_5_21023(x) }
fn n_5_21030(x: i32) -> i32 { x.wrapping_add(788) }
fn n_5_21031(x: i32) -> i32 { x.wrapping_add(789) }
fn n_5_21032(x: i32) -> i32 { x.wrapping_add(790) }
fn n_5_21033(x: i32) -> i32 { x.wrapping_add(791) }
fn n_4_2103(x: i32) -> i32 { n_5_21030(x) + n_5_21031(x) + n_5_21032(x) + n_5_21033(x) }
fn n_3_210(x: i32) -> i32 { n_4_2100(x) + n_4_2101(x) + n_4_2102(x) + n_4_2103(x) }
fn n_5_21100(x: i32) -> i32 { x.wrapping_add(794) }
fn n_5_21101(x: i32) -> i32 { x.wrapping_add(795) }
fn n_5_21102(x: i32) -> i32 { x.wrapping_add(796) }
fn n_5_21103(x: i32) -> i32 { x.wrapping_add(797) }
fn n_4_2110(x: i32) -> i32 { n_5_21100(x) + n_5_21101(x) + n_5_21102(x) + n_5_21103(x) }
fn n_5_21110(x: i32) -> i32 { x.wrapping_add(799) }
fn n_5_21111(x: i32) -> i32 { x.wrapping_add(800) }
fn n_5_21112(x: i32) -> i32 { x.wrapping_add(801) }
fn n_5_21113(x: i32) -> i32 { x.wrapping_add(802) }
fn n_4_2111(x: i32) -> i32 { n_5_21110(x) + n_5_21111(x) + n_5_21112(x) + n_5_21113(x) }
fn n_5_21120(x: i32) -> i32 { x.wrapping_add(804) }
fn n_5_21121(x: i32) -> i32 { x.wrapping_add(805) }
fn n_5_21122(x: i32) -> i32 { x.wrapping_add(806) }
fn n_5_21123(x: i32) -> i32 { x.wrapping_add(807) }
fn n_4_2112(x: i32) -> i32 { n_5_21120(x) + n_5_21121(x) + n_5_21122(x) + n_5_21123(x) }
fn n_5_21130(x: i32) -> i32 { x.wrapping_add(809) }
fn n_5_21131(x: i32) -> i32 { x.wrapping_add(810) }
fn n_5_21132(x: i32) -> i32 { x.wrapping_add(811) }
fn n_5_21133(x: i32) -> i32 { x.wrapping_add(812) }
fn n_4_2113(x: i32) -> i32 { n_5_21130(x) + n_5_21131(x) + n_5_21132(x) + n_5_21133(x) }
fn n_3_211(x: i32) -> i32 { n_4_2110(x) + n_4_2111(x) + n_4_2112(x) + n_4_2113(x) }
fn n_5_21200(x: i32) -> i32 { x.wrapping_add(815) }
fn n_5_21201(x: i32) -> i32 { x.wrapping_add(816) }
fn n_5_21202(x: i32) -> i32 { x.wrapping_add(817) }
fn n_5_21203(x: i32) -> i32 { x.wrapping_add(818) }
fn n_4_2120(x: i32) -> i32 { n_5_21200(x) + n_5_21201(x) + n_5_21202(x) + n_5_21203(x) }
fn n_5_21210(x: i32) -> i32 { x.wrapping_add(820) }
fn n_5_21211(x: i32) -> i32 { x.wrapping_add(821) }
fn n_5_21212(x: i32) -> i32 { x.wrapping_add(822) }
fn n_5_21213(x: i32) -> i32 { x.wrapping_add(823) }
fn n_4_2121(x: i32) -> i32 { n_5_21210(x) + n_5_21211(x) + n_5_21212(x) + n_5_21213(x) }
fn n_5_21220(x: i32) -> i32 { x.wrapping_add(825) }
fn n_5_21221(x: i32) -> i32 { x.wrapping_add(826) }
fn n_5_21222(x: i32) -> i32 { x.wrapping_add(827) }
fn n_5_21223(x: i32) -> i32 { x.wrapping_add(828) }
fn n_4_2122(x: i32) -> i32 { n_5_21220(x) + n_5_21221(x) + n_5_21222(x) + n_5_21223(x) }
fn n_5_21230(x: i32) -> i32 { x.wrapping_add(830) }
fn n_5_21231(x: i32) -> i32 { x.wrapping_add(831) }
fn n_5_21232(x: i32) -> i32 { x.wrapping_add(832) }
fn n_5_21233(x: i32) -> i32 { x.wrapping_add(833) }
fn n_4_2123(x: i32) -> i32 { n_5_21230(x) + n_5_21231(x) + n_5_21232(x) + n_5_21233(x) }
fn n_3_212(x: i32) -> i32 { n_4_2120(x) + n_4_2121(x) + n_4_2122(x) + n_4_2123(x) }
fn n_5_21300(x: i32) -> i32 { x.wrapping_add(836) }
fn n_5_21301(x: i32) -> i32 { x.wrapping_add(837) }
fn n_5_21302(x: i32) -> i32 { x.wrapping_add(838) }
fn n_5_21303(x: i32) -> i32 { x.wrapping_add(839) }
fn n_4_2130(x: i32) -> i32 { n_5_21300(x) + n_5_21301(x) + n_5_21302(x) + n_5_21303(x) }
fn n_5_21310(x: i32) -> i32 { x.wrapping_add(841) }
fn n_5_21311(x: i32) -> i32 { x.wrapping_add(842) }
fn n_5_21312(x: i32) -> i32 { x.wrapping_add(843) }
fn n_5_21313(x: i32) -> i32 { x.wrapping_add(844) }
fn n_4_2131(x: i32) -> i32 { n_5_21310(x) + n_5_21311(x) + n_5_21312(x) + n_5_21313(x) }
fn n_5_21320(x: i32) -> i32 { x.wrapping_add(846) }
fn n_5_21321(x: i32) -> i32 { x.wrapping_add(847) }
fn n_5_21322(x: i32) -> i32 { x.wrapping_add(848) }
fn n_5_21323(x: i32) -> i32 { x.wrapping_add(849) }
fn n_4_2132(x: i32) -> i32 { n_5_21320(x) + n_5_21321(x) + n_5_21322(x) + n_5_21323(x) }
fn n_5_21330(x: i32) -> i32 { x.wrapping_add(851) }
fn n_5_21331(x: i32) -> i32 { x.wrapping_add(852) }
fn n_5_21332(x: i32) -> i32 { x.wrapping_add(853) }
fn n_5_21333(x: i32) -> i32 { x.wrapping_add(854) }
fn n_4_2133(x: i32) -> i32 { n_5_21330(x) + n_5_21331(x) + n_5_21332(x) + n_5_21333(x) }
fn n_3_213(x: i32) -> i32 { n_4_2130(x) + n_4_2131(x) + n_4_2132(x) + n_4_2133(x) }
fn n_2_21(x: i32) -> i32 { n_3_210(x) + n_3_211(x) + n_3_212(x) + n_3_213(x) }
fn n_5_22000(x: i32) -> i32 { x.wrapping_add(858) }
fn n_5_22001(x: i32) -> i32 { x.wrapping_add(859) }
fn n_5_22002(x: i32) -> i32 { x.wrapping_add(860) }
fn n_5_22003(x: i32) -> i32 { x.wrapping_add(861) }
fn n_4_2200(x: i32) -> i32 { n_5_22000(x) + n_5_22001(x) + n_5_22002(x) + n_5_22003(x) }
fn n_5_22010(x: i32) -> i32 { x.wrapping_add(863) }
fn n_5_22011(x: i32) -> i32 { x.wrapping_add(864) }
fn n_5_22012(x: i32) -> i32 { x.wrapping_add(865) }
fn n_5_22013(x: i32) -> i32 { x.wrapping_add(866) }
fn n_4_2201(x: i32) -> i32 { n_5_22010(x) + n_5_22011(x) + n_5_22012(x) + n_5_22013(x) }
fn n_5_22020(x: i32) -> i32 { x.wrapping_add(868) }
fn n_5_22021(x: i32) -> i32 { x.wrapping_add(869) }
fn n_5_22022(x: i32) -> i32 { x.wrapping_add(870) }
fn n_5_22023(x: i32) -> i32 { x.wrapping_add(871) }
fn n_4_2202(x: i32) -> i32 { n_5_22020(x) + n_5_22021(x) + n_5_22022(x) + n_5_22023(x) }
fn n_5_22030(x: i32) -> i32 { x.wrapping_add(873) }
fn n_5_22031(x: i32) -> i32 { x.wrapping_add(874) }
fn n_5_22032(x: i32) -> i32 { x.wrapping_add(875) }
fn n_5_22033(x: i32) -> i32 { x.wrapping_add(876) }
fn n_4_2203(x: i32) -> i32 { n_5_22030(x) + n_5_22031(x) + n_5_22032(x) + n_5_22033(x) }
fn n_3_220(x: i32) -> i32 { n_4_2200(x) + n_4_2201(x) + n_4_2202(x) + n_4_2203(x) }
fn n_5_22100(x: i32) -> i32 { x.wrapping_add(879) }
fn n_5_22101(x: i32) -> i32 { x.wrapping_add(880) }
fn n_5_22102(x: i32) -> i32 { x.wrapping_add(881) }
fn n_5_22103(x: i32) -> i32 { x.wrapping_add(882) }
fn n_4_2210(x: i32) -> i32 { n_5_22100(x) + n_5_22101(x) + n_5_22102(x) + n_5_22103(x) }
fn n_5_22110(x: i32) -> i32 { x.wrapping_add(884) }
fn n_5_22111(x: i32) -> i32 { x.wrapping_add(885) }
fn n_5_22112(x: i32) -> i32 { x.wrapping_add(886) }
fn n_5_22113(x: i32) -> i32 { x.wrapping_add(887) }
fn n_4_2211(x: i32) -> i32 { n_5_22110(x) + n_5_22111(x) + n_5_22112(x) + n_5_22113(x) }
fn n_5_22120(x: i32) -> i32 { x.wrapping_add(889) }
fn n_5_22121(x: i32) -> i32 { x.wrapping_add(890) }
fn n_5_22122(x: i32) -> i32 { x.wrapping_add(891) }
fn n_5_22123(x: i32) -> i32 { x.wrapping_add(892) }
fn n_4_2212(x: i32) -> i32 { n_5_22120(x) + n_5_22121(x) + n_5_22122(x) + n_5_22123(x) }
fn n_5_22130(x: i32) -> i32 { x.wrapping_add(894) }
fn n_5_22131(x: i32) -> i32 { x.wrapping_add(895) }
fn n_5_22132(x: i32) -> i32 { x.wrapping_add(896) }
fn n_5_22133(x: i32) -> i32 { x.wrapping_add(897) }
fn n_4_2213(x: i32) -> i32 { n_5_22130(x) + n_5_22131(x) + n_5_22132(x) + n_5_22133(x) }
fn n_3_221(x: i32) -> i32 { n_4_2210(x) + n_4_2211(x) + n_4_2212(x) + n_4_2213(x) }
fn n_5_22200(x: i32) -> i32 { x.wrapping_add(900) }
fn n_5_22201(x: i32) -> i32 { x.wrapping_add(901) }
fn n_5_22202(x: i32) -> i32 { x.wrapping_add(902) }
fn n_5_22203(x: i32) -> i32 { x.wrapping_add(903) }
fn n_4_2220(x: i32) -> i32 { n_5_22200(x) + n_5_22201(x) + n_5_22202(x) + n_5_22203(x) }
fn n_5_22210(x: i32) -> i32 { x.wrapping_add(905) }
fn n_5_22211(x: i32) -> i32 { x.wrapping_add(906) }
fn n_5_22212(x: i32) -> i32 { x.wrapping_add(907) }
fn n_5_22213(x: i32) -> i32 { x.wrapping_add(908) }
fn n_4_2221(x: i32) -> i32 { n_5_22210(x) + n_5_22211(x) + n_5_22212(x) + n_5_22213(x) }
fn n_5_22220(x: i32) -> i32 { x.wrapping_add(910) }
fn n_5_22221(x: i32) -> i32 { x.wrapping_add(911) }
fn n_5_22222(x: i32) -> i32 { x.wrapping_add(912) }
fn n_5_22223(x: i32) -> i32 { x.wrapping_add(913) }
fn n_4_2222(x: i32) -> i32 { n_5_22220(x) + n_5_22221(x) + n_5_22222(x) + n_5_22223(x) }
fn n_5_22230(x: i32) -> i32 { x.wrapping_add(915) }
fn n_5_22231(x: i32) -> i32 { x.wrapping_add(916) }
fn n_5_22232(x: i32) -> i32 { x.wrapping_add(917) }
fn n_5_22233(x: i32) -> i32 { x.wrapping_add(918) }
fn n_4_2223(x: i32) -> i32 { n_5_22230(x) + n_5_22231(x) + n_5_22232(x) + n_5_22233(x) }
fn n_3_222(x: i32) -> i32 { n_4_2220(x) + n_4_2221(x) + n_4_2222(x) + n_4_2223(x) }
fn n_5_22300(x: i32) -> i32 { x.wrapping_add(921) }
fn n_5_22301(x: i32) -> i32 { x.wrapping_add(922) }
fn n_5_22302(x: i32) -> i32 { x.wrapping_add(923) }
fn n_5_22303(x: i32) -> i32 { x.wrapping_add(924) }
fn n_4_2230(x: i32) -> i32 { n_5_22300(x) + n_5_22301(x) + n_5_22302(x) + n_5_22303(x) }
fn n_5_22310(x: i32) -> i32 { x.wrapping_add(926) }
fn n_5_22311(x: i32) -> i32 { x.wrapping_add(927) }
fn n_5_22312(x: i32) -> i32 { x.wrapping_add(928) }
fn n_5_22313(x: i32) -> i32 { x.wrapping_add(929) }
fn n_4_2231(x: i32) -> i32 { n_5_22310(x) + n_5_22311(x) + n_5_22312(x) + n_5_22313(x) }
fn n_5_22320(x: i32) -> i32 { x.wrapping_add(931) }
fn n_5_22321(x: i32) -> i32 { x.wrapping_add(932) }
fn n_5_22322(x: i32) -> i32 { x.wrapping_add(933) }
fn n_5_22323(x: i32) -> i32 { x.wrapping_add(934) }
fn n_4_2232(x: i32) -> i32 { n_5_22320(x) + n_5_22321(x) + n_5_22322(x) + n_5_22323(x) }
fn n_5_22330(x: i32) -> i32 { x.wrapping_add(936) }
fn n_5_22331(x: i32) -> i32 { x.wrapping_add(937) }
fn n_5_22332(x: i32) -> i32 { x.wrapping_add(938) }
fn n_5_22333(x: i32) -> i32 { x.wrapping_add(939) }
fn n_4_2233(x: i32) -> i32 { n_5_22330(x) + n_5_22331(x) + n_5_22332(x) + n_5_22333(x) }
fn n_3_223(x: i32) -> i32 { n_4_2230(x) + n_4_2231(x) + n_4_2232(x) + n_4_2233(x) }
fn n_2_22(x: i32) -> i32 { n_3_220(x) + n_3_221(x) + n_3_222(x) + n_3_223(x) }
fn n_5_23000(x: i32) -> i32 { x.wrapping_add(943) }
fn n_5_23001(x: i32) -> i32 { x.wrapping_add(944) }
fn n_5_23002(x: i32) -> i32 { x.wrapping_add(945) }
fn n_5_23003(x: i32) -> i32 { x.wrapping_add(946) }
fn n_4_2300(x: i32) -> i32 { n_5_23000(x) + n_5_23001(x) + n_5_23002(x) + n_5_23003(x) }
fn n_5_23010(x: i32) -> i32 { x.wrapping_add(948) }
fn n_5_23011(x: i32) -> i32 { x.wrapping_add(949) }
fn n_5_23012(x: i32) -> i32 { x.wrapping_add(950) }
fn n_5_23013(x: i32) -> i32 { x.wrapping_add(951) }
fn n_4_2301(x: i32) -> i32 { n_5_23010(x) + n_5_23011(x) + n_5_23012(x) + n_5_23013(x) }
fn n_5_23020(x: i32) -> i32 { x.wrapping_add(953) }
fn n_5_23021(x: i32) -> i32 { x.wrapping_add(954) }
fn n_5_23022(x: i32) -> i32 { x.wrapping_add(955) }
fn n_5_23023(x: i32) -> i32 { x.wrapping_add(956) }
fn n_4_2302(x: i32) -> i32 { n_5_23020(x) + n_5_23021(x) + n_5_23022(x) + n_5_23023(x) }
fn n_5_23030(x: i32) -> i32 { x.wrapping_add(958) }
fn n_5_23031(x: i32) -> i32 { x.wrapping_add(959) }
fn n_5_23032(x: i32) -> i32 { x.wrapping_add(960) }
fn n_5_23033(x: i32) -> i32 { x.wrapping_add(961) }
fn n_4_2303(x: i32) -> i32 { n_5_23030(x) + n_5_23031(x) + n_5_23032(x) + n_5_23033(x) }
fn n_3_230(x: i32) -> i32 { n_4_2300(x) + n_4_2301(x) + n_4_2302(x) + n_4_2303(x) }
fn n_5_23100(x: i32) -> i32 { x.wrapping_add(964) }
fn n_5_23101(x: i32) -> i32 { x.wrapping_add(965) }
fn n_5_23102(x: i32) -> i32 { x.wrapping_add(966) }
fn n_5_23103(x: i32) -> i32 { x.wrapping_add(967) }
fn n_4_2310(x: i32) -> i32 { n_5_23100(x) + n_5_23101(x) + n_5_23102(x) + n_5_23103(x) }
fn n_5_23110(x: i32) -> i32 { x.wrapping_add(969) }
fn n_5_23111(x: i32) -> i32 { x.wrapping_add(970) }
fn n_5_23112(x: i32) -> i32 { x.wrapping_add(971) }
fn n_5_23113(x: i32) -> i32 { x.wrapping_add(972) }
fn n_4_2311(x: i32) -> i32 { n_5_23110(x) + n_5_23111(x) + n_5_23112(x) + n_5_23113(x) }
fn n_5_23120(x: i32) -> i32 { x.wrapping_add(974) }
fn n_5_23121(x: i32) -> i32 { x.wrapping_add(975) }
fn n_5_23122(x: i32) -> i32 { x.wrapping_add(976) }
fn n_5_23123(x: i32) -> i32 { x.wrapping_add(977) }
fn n_4_2312(x: i32) -> i32 { n_5_23120(x) + n_5_23121(x) + n_5_23122(x) + n_5_23123(x) }
fn n_5_23130(x: i32) -> i32 { x.wrapping_add(979) }
fn n_5_23131(x: i32) -> i32 { x.wrapping_add(980) }
fn n_5_23132(x: i32) -> i32 { x.wrapping_add(981) }
fn n_5_23133(x: i32) -> i32 { x.wrapping_add(982) }
fn n_4_2313(x: i32) -> i32 { n_5_23130(x) + n_5_23131(x) + n_5_23132(x) + n_5_23133(x) }
fn n_3_231(x: i32) -> i32 { n_4_2310(x) + n_4_2311(x) + n_4_2312(x) + n_4_2313(x) }
fn n_5_23200(x: i32) -> i32 { x.wrapping_add(985) }
fn n_5_23201(x: i32) -> i32 { x.wrapping_add(986) }
fn n_5_23202(x: i32) -> i32 { x.wrapping_add(987) }
fn n_5_23203(x: i32) -> i32 { x.wrapping_add(988) }
fn n_4_2320(x: i32) -> i32 { n_5_23200(x) + n_5_23201(x) + n_5_23202(x) + n_5_23203(x) }
fn n_5_23210(x: i32) -> i32 { x.wrapping_add(990) }
fn n_5_23211(x: i32) -> i32 { x.wrapping_add(991) }
fn n_5_23212(x: i32) -> i32 { x.wrapping_add(992) }
fn n_5_23213(x: i32) -> i32 { x.wrapping_add(993) }
fn n_4_2321(x: i32) -> i32 { n_5_23210(x) + n_5_23211(x) + n_5_23212(x) + n_5_23213(x) }
fn n_5_23220(x: i32) -> i32 { x.wrapping_add(995) }
fn n_5_23221(x: i32) -> i32 { x.wrapping_add(996) }
fn n_5_23222(x: i32) -> i32 { x.wrapping_add(997) }
fn n_5_23223(x: i32) -> i32 { x.wrapping_add(998) }
fn n_4_2322(x: i32) -> i32 { n_5_23220(x) + n_5_23221(x) + n_5_23222(x) + n_5_23223(x) }
fn n_5_23230(x: i32) -> i32 { x.wrapping_add(1000) }
fn n_5_23231(x: i32) -> i32 { x.wrapping_add(1001) }
fn n_5_23232(x: i32) -> i32 { x.wrapping_add(1002) }
fn n_5_23233(x: i32) -> i32 { x.wrapping_add(1003) }
fn n_4_2323(x: i32) -> i32 { n_5_23230(x) + n_5_23231(x) + n_5_23232(x) + n_5_23233(x) }
fn n_3_232(x: i32) -> i32 { n_4_2320(x) + n_4_2321(x) + n_4_2322(x) + n_4_2323(x) }
fn n_5_23300(x: i32) -> i32 { x.wrapping_add(1006) }
fn n_5_23301(x: i32) -> i32 { x.wrapping_add(1007) }
fn n_5_23302(x: i32) -> i32 { x.wrapping_add(1008) }
fn n_5_23303(x: i32) -> i32 { x.wrapping_add(1009) }
fn n_4_2330(x: i32) -> i32 { n_5_23300(x) + n_5_23301(x) + n_5_23302(x) + n_5_23303(x) }
fn n_5_23310(x: i32) -> i32 { x.wrapping_add(1011) }
fn n_5_23311(x: i32) -> i32 { x.wrapping_add(1012) }
fn n_5_23312(x: i32) -> i32 { x.wrapping_add(1013) }
fn n_5_23313(x: i32) -> i32 { x.wrapping_add(1014) }
fn n_4_2331(x: i32) -> i32 { n_5_23310(x) + n_5_23311(x) + n_5_23312(x) + n_5_23313(x) }
fn n_5_23320(x: i32) -> i32 { x.wrapping_add(1016) }
fn n_5_23321(x: i32) -> i32 { x.wrapping_add(1017) }
fn n_5_23322(x: i32) -> i32 { x.wrapping_add(1018) }
fn n_5_23323(x: i32) -> i32 { x.wrapping_add(1019) }
fn n_4_2332(x: i32) -> i32 { n_5_23320(x) + n_5_23321(x) + n_5_23322(x) + n_5_23323(x) }
fn n_5_23330(x: i32) -> i32 { x.wrapping_add(1021) }
fn n_5_23331(x: i32) -> i32 { x.wrapping_add(1022) }
fn n_5_23332(x: i32) -> i32 { x.wrapping_add(1023) }
fn n_5_23333(x: i32) -> i32 { x.wrapping_add(1024) }
fn n_4_2333(x: i32) -> i32 { n_5_23330(x) + n_5_23331(x) + n_5_23332(x) + n_5_23333(x) }
fn n_3_233(x: i32) -> i32 { n_4_2330(x) + n_4_2331(x) + n_4_2332(x) + n_4_2333(x) }
fn n_2_23(x: i32) -> i32 { n_3_230(x) + n_3_231(x) + n_3_232(x) + n_3_233(x) }
fn n_1_2(x: i32) -> i32 { n_2_20(x) + n_2_21(x) + n_2_22(x) + n_2_23(x) }
fn n_5_30000(x: i32) -> i32 { x.wrapping_add(1029) }
fn n_5_30001(x: i32) -> i32 { x.wrapping_add(1030) }
fn n_5_30002(x: i32) -> i32 { x.wrapping_add(1031) }
fn n_5_30003(x: i32) -> i32 { x.wrapping_add(1032) }
fn n_4_3000(x: i32) -> i32 { n_5_30000(x) + n_5_30001(x) + n_5_30002(x) + n_5_30003(x) }
fn n_5_30010(x: i32) -> i32 { x.wrapping_add(1034) }
fn n_5_30011(x: i32) -> i32 { x.wrapping_add(1035) }
fn n_5_30012(x: i32) -> i32 { x.wrapping_add(1036) }
fn n_5_30013(x: i32) -> i32 { x.wrapping_add(1037) }
fn n_4_3001(x: i32) -> i32 { n_5_30010(x) + n_5_30011(x) + n_5_30012(x) + n_5_30013(x) }
fn n_5_30020(x: i32) -> i32 { x.wrapping_add(1039) }
fn n_5_30021(x: i32) -> i32 { x.wrapping_add(1040) }
fn n_5_30022(x: i32) -> i32 { x.wrapping_add(1041) }
fn n_5_30023(x: i32) -> i32 { x.wrapping_add(1042) }
fn n_4_3002(x: i32) -> i32 { n_5_30020(x) + n_5_30021(x) + n_5_30022(x) + n_5_30023(x) }
fn n_5_30030(x: i32) -> i32 { x.wrapping_add(1044) }
fn n_5_30031(x: i32) -> i32 { x.wrapping_add(1045) }
fn n_5_30032(x: i32) -> i32 { x.wrapping_add(1046) }
fn n_5_30033(x: i32) -> i32 { x.wrapping_add(1047) }
fn n_4_3003(x: i32) -> i32 { n_5_30030(x) + n_5_30031(x) + n_5_30032(x) + n_5_30033(x) }
fn n_3_300(x: i32) -> i32 { n_4_3000(x) + n_4_3001(x) + n_4_3002(x) + n_4_3003(x) }
fn n_5_30100(x: i32) -> i32 { x.wrapping_add(1050) }
fn n_5_30101(x: i32) -> i32 { x.wrapping_add(1051) }
fn n_5_30102(x: i32) -> i32 { x.wrapping_add(1052) }
fn n_5_30103(x: i32) -> i32 { x.wrapping_add(1053) }
fn n_4_3010(x: i32) -> i32 { n_5_30100(x) + n_5_30101(x) + n_5_30102(x) + n_5_30103(x) }
fn n_5_30110(x: i32) -> i32 { x.wrapping_add(1055) }
fn n_5_30111(x: i32) -> i32 { x.wrapping_add(1056) }
fn n_5_30112(x: i32) -> i32 { x.wrapping_add(1057) }
fn n_5_30113(x: i32) -> i32 { x.wrapping_add(1058) }
fn n_4_3011(x: i32) -> i32 { n_5_30110(x) + n_5_30111(x) + n_5_30112(x) + n_5_30113(x) }
fn n_5_30120(x: i32) -> i32 { x.wrapping_add(1060) }
fn n_5_30121(x: i32) -> i32 { x.wrapping_add(1061) }
fn n_5_30122(x: i32) -> i32 { x.wrapping_add(1062) }
fn n_5_30123(x: i32) -> i32 { x.wrapping_add(1063) }
fn n_4_3012(x: i32) -> i32 { n_5_30120(x) + n_5_30121(x) + n_5_30122(x) + n_5_30123(x) }
fn n_5_30130(x: i32) -> i32 { x.wrapping_add(1065) }
fn n_5_30131(x: i32) -> i32 { x.wrapping_add(1066) }
fn n_5_30132(x: i32) -> i32 { x.wrapping_add(1067) }
fn n_5_30133(x: i32) -> i32 { x.wrapping_add(1068) }
fn n_4_3013(x: i32) -> i32 { n_5_30130(x) + n_5_30131(x) + n_5_30132(x) + n_5_30133(x) }
fn n_3_301(x: i32) -> i32 { n_4_3010(x) + n_4_3011(x) + n_4_3012(x) + n_4_3013(x) }
fn n_5_30200(x: i32) -> i32 { x.wrapping_add(1071) }
fn n_5_30201(x: i32) -> i32 { x.wrapping_add(1072) }
fn n_5_30202(x: i32) -> i32 { x.wrapping_add(1073) }
fn n_5_30203(x: i32) -> i32 { x.wrapping_add(1074) }
fn n_4_3020(x: i32) -> i32 { n_5_30200(x) + n_5_30201(x) + n_5_30202(x) + n_5_30203(x) }
fn n_5_30210(x: i32) -> i32 { x.wrapping_add(1076) }
fn n_5_30211(x: i32) -> i32 { x.wrapping_add(1077) }
fn n_5_30212(x: i32) -> i32 { x.wrapping_add(1078) }
fn n_5_30213(x: i32) -> i32 { x.wrapping_add(1079) }
fn n_4_3021(x: i32) -> i32 { n_5_30210(x) + n_5_30211(x) + n_5_30212(x) + n_5_30213(x) }
fn n_5_30220(x: i32) -> i32 { x.wrapping_add(1081) }
fn n_5_30221(x: i32) -> i32 { x.wrapping_add(1082) }
fn n_5_30222(x: i32) -> i32 { x.wrapping_add(1083) }
fn n_5_30223(x: i32) -> i32 { x.wrapping_add(1084) }
fn n_4_3022(x: i32) -> i32 { n_5_30220(x) + n_5_30221(x) + n_5_30222(x) + n_5_30223(x) }
fn n_5_30230(x: i32) -> i32 { x.wrapping_add(1086) }
fn n_5_30231(x: i32) -> i32 { x.wrapping_add(1087) }
fn n_5_30232(x: i32) -> i32 { x.wrapping_add(1088) }
fn n_5_30233(x: i32) -> i32 { x.wrapping_add(1089) }
fn n_4_3023(x: i32) -> i32 { n_5_30230(x) + n_5_30231(x) + n_5_30232(x) + n_5_30233(x) }
fn n_3_302(x: i32) -> i32 { n_4_3020(x) + n_4_3021(x) + n_4_3022(x) + n_4_3023(x) }
fn n_5_30300(x: i32) -> i32 { x.wrapping_add(1092) }
fn n_5_30301(x: i32) -> i32 { x.wrapping_add(1093) }
fn n_5_30302(x: i32) -> i32 { x.wrapping_add(1094) }
fn n_5_30303(x: i32) -> i32 { x.wrapping_add(1095) }
fn n_4_3030(x: i32) -> i32 { n_5_30300(x) + n_5_30301(x) + n_5_30302(x) + n_5_30303(x) }
fn n_5_30310(x: i32) -> i32 { x.wrapping_add(1097) }
fn n_5_30311(x: i32) -> i32 { x.wrapping_add(1098) }
fn n_5_30312(x: i32) -> i32 { x.wrapping_add(1099) }
fn n_5_30313(x: i32) -> i32 { x.wrapping_add(1100) }
fn n_4_3031(x: i32) -> i32 { n_5_30310(x) + n_5_30311(x) + n_5_30312(x) + n_5_30313(x) }
fn n_5_30320(x: i32) -> i32 { x.wrapping_add(1102) }
fn n_5_30321(x: i32) -> i32 { x.wrapping_add(1103) }
fn n_5_30322(x: i32) -> i32 { x.wrapping_add(1104) }
fn n_5_30323(x: i32) -> i32 { x.wrapping_add(1105) }
fn n_4_3032(x: i32) -> i32 { n_5_30320(x) + n_5_30321(x) + n_5_30322(x) + n_5_30323(x) }
fn n_5_30330(x: i32) -> i32 { x.wrapping_add(1107) }
fn n_5_30331(x: i32) -> i32 { x.wrapping_add(1108) }
fn n_5_30332(x: i32) -> i32 { x.wrapping_add(1109) }
fn n_5_30333(x: i32) -> i32 { x.wrapping_add(1110) }
fn n_4_3033(x: i32) -> i32 { n_5_30330(x) + n_5_30331(x) + n_5_30332(x) + n_5_30333(x) }
fn n_3_303(x: i32) -> i32 { n_4_3030(x) + n_4_3031(x) + n_4_3032(x) + n_4_3033(x) }
fn n_2_30(x: i32) -> i32 { n_3_300(x) + n_3_301(x) + n_3_302(x) + n_3_303(x) }
fn n_5_31000(x: i32) -> i32 { x.wrapping_add(1114) }
fn n_5_31001(x: i32) -> i32 { x.wrapping_add(1115) }
fn n_5_31002(x: i32) -> i32 { x.wrapping_add(1116) }
fn n_5_31003(x: i32) -> i32 { x.wrapping_add(1117) }
fn n_4_3100(x: i32) -> i32 { n_5_31000(x) + n_5_31001(x) + n_5_31002(x) + n_5_31003(x) }
fn n_5_31010(x: i32) -> i32 { x.wrapping_add(1119) }
fn n_5_31011(x: i32) -> i32 { x.wrapping_add(1120) }
fn n_5_31012(x: i32) -> i32 { x.wrapping_add(1121) }
fn n_5_31013(x: i32) -> i32 { x.wrapping_add(1122) }
fn n_4_3101(x: i32) -> i32 { n_5_31010(x) + n_5_31011(x) + n_5_31012(x) + n_5_31013(x) }
fn n_5_31020(x: i32) -> i32 { x.wrapping_add(1124) }
fn n_5_31021(x: i32) -> i32 { x.wrapping_add(1125) }
fn n_5_31022(x: i32) -> i32 { x.wrapping_add(1126) }
fn n_5_31023(x: i32) -> i32 { x.wrapping_add(1127) }
fn n_4_3102(x: i32) -> i32 { n_5_31020(x) + n_5_31021(x) + n_5_31022(x) + n_5_31023(x) }
fn n_5_31030(x: i32) -> i32 { x.wrapping_add(1129) }
fn n_5_31031(x: i32) -> i32 { x.wrapping_add(1130) }
fn n_5_31032(x: i32) -> i32 { x.wrapping_add(1131) }
fn n_5_31033(x: i32) -> i32 { x.wrapping_add(1132) }
fn n_4_3103(x: i32) -> i32 { n_5_31030(x) + n_5_31031(x) + n_5_31032(x) + n_5_31033(x) }
fn n_3_310(x: i32) -> i32 { n_4_3100(x) + n_4_3101(x) + n_4_3102(x) + n_4_3103(x) }
fn n_5_31100(x: i32) -> i32 { x.wrapping_add(1135) }
fn n_5_31101(x: i32) -> i32 { x.wrapping_add(1136) }
fn n_5_31102(x: i32) -> i32 { x.wrapping_add(1137) }
fn n_5_31103(x: i32) -> i32 { x.wrapping_add(1138) }
fn n_4_3110(x: i32) -> i32 { n_5_31100(x) + n_5_31101(x) + n_5_31102(x) + n_5_31103(x) }
fn n_5_31110(x: i32) -> i32 { x.wrapping_add(1140) }
fn n_5_31111(x: i32) -> i32 { x.wrapping_add(1141) }
fn n_5_31112(x: i32) -> i32 { x.wrapping_add(1142) }
fn n_5_31113(x: i32) -> i32 { x.wrapping_add(1143) }
fn n_4_3111(x: i32) -> i32 { n_5_31110(x) + n_5_31111(x) + n_5_31112(x) + n_5_31113(x) }
fn n_5_31120(x: i32) -> i32 { x.wrapping_add(1145) }
fn n_5_31121(x: i32) -> i32 { x.wrapping_add(1146) }
fn n_5_31122(x: i32) -> i32 { x.wrapping_add(1147) }
fn n_5_31123(x: i32) -> i32 { x.wrapping_add(1148) }
fn n_4_3112(x: i32) -> i32 { n_5_31120(x) + n_5_31121(x) + n_5_31122(x) + n_5_31123(x) }
fn n_5_31130(x: i32) -> i32 { x.wrapping_add(1150) }
fn n_5_31131(x: i32) -> i32 { x.wrapping_add(1151) }
fn n_5_31132(x: i32) -> i32 { x.wrapping_add(1152) }
fn n_5_31133(x: i32) -> i32 { x.wrapping_add(1153) }
fn n_4_3113(x: i32) -> i32 { n_5_31130(x) + n_5_31131(x) + n_5_31132(x) + n_5_31133(x) }
fn n_3_311(x: i32) -> i32 { n_4_3110(x) + n_4_3111(x) + n_4_3112(x) + n_4_3113(x) }
fn n_5_31200(x: i32) -> i32 { x.wrapping_add(1156) }
fn n_5_31201(x: i32) -> i32 { x.wrapping_add(1157) }
fn n_5_31202(x: i32) -> i32 { x.wrapping_add(1158) }
fn n_5_31203(x: i32) -> i32 { x.wrapping_add(1159) }
fn n_4_3120(x: i32) -> i32 { n_5_31200(x) + n_5_31201(x) + n_5_31202(x) + n_5_31203(x) }
fn n_5_31210(x: i32) -> i32 { x.wrapping_add(1161) }
fn n_5_31211(x: i32) -> i32 { x.wrapping_add(1162) }
fn n_5_31212(x: i32) -> i32 { x.wrapping_add(1163) }
fn n_5_31213(x: i32) -> i32 { x.wrapping_add(1164) }
fn n_4_3121(x: i32) -> i32 { n_5_31210(x) + n_5_31211(x) + n_5_31212(x) + n_5_31213(x) }
fn n_5_31220(x: i32) -> i32 { x.wrapping_add(1166) }
fn n_5_31221(x: i32) -> i32 { x.wrapping_add(1167) }
fn n_5_31222(x: i32) -> i32 { x.wrapping_add(1168) }
fn n_5_31223(x: i32) -> i32 { x.wrapping_add(1169) }
fn n_4_3122(x: i32) -> i32 { n_5_31220(x) + n_5_31221(x) + n_5_31222(x) + n_5_31223(x) }
fn n_5_31230(x: i32) -> i32 { x.wrapping_add(1171) }
fn n_5_31231(x: i32) -> i32 { x.wrapping_add(1172) }
fn n_5_31232(x: i32) -> i32 { x.wrapping_add(1173) }
fn n_5_31233(x: i32) -> i32 { x.wrapping_add(1174) }
fn n_4_3123(x: i32) -> i32 { n_5_31230(x) + n_5_31231(x) + n_5_31232(x) + n_5_31233(x) }
fn n_3_312(x: i32) -> i32 { n_4_3120(x) + n_4_3121(x) + n_4_3122(x) + n_4_3123(x) }
fn n_5_31300(x: i32) -> i32 { x.wrapping_add(1177) }
fn n_5_31301(x: i32) -> i32 { x.wrapping_add(1178) }
fn n_5_31302(x: i32) -> i32 { x.wrapping_add(1179) }
fn n_5_31303(x: i32) -> i32 { x.wrapping_add(1180) }
fn n_4_3130(x: i32) -> i32 { n_5_31300(x) + n_5_31301(x) + n_5_31302(x) + n_5_31303(x) }
fn n_5_31310(x: i32) -> i32 { x.wrapping_add(1182) }
fn n_5_31311(x: i32) -> i32 { x.wrapping_add(1183) }
fn n_5_31312(x: i32) -> i32 { x.wrapping_add(1184) }
fn n_5_31313(x: i32) -> i32 { x.wrapping_add(1185) }
fn n_4_3131(x: i32) -> i32 { n_5_31310(x) + n_5_31311(x) + n_5_31312(x) + n_5_31313(x) }
fn n_5_31320(x: i32) -> i32 { x.wrapping_add(1187) }
fn n_5_31321(x: i32) -> i32 { x.wrapping_add(1188) }
fn n_5_31322(x: i32) -> i32 { x.wrapping_add(1189) }
fn n_5_31323(x: i32) -> i32 { x.wrapping_add(1190) }
fn n_4_3132(x: i32) -> i32 { n_5_31320(x) + n_5_31321(x) + n_5_31322(x) + n_5_31323(x) }
fn n_5_31330(x: i32) -> i32 { x.wrapping_add(1192) }
fn n_5_31331(x: i32) -> i32 { x.wrapping_add(1193) }
fn n_5_31332(x: i32) -> i32 { x.wrapping_add(1194) }
fn n_5_31333(x: i32) -> i32 { x.wrapping_add(1195) }
fn n_4_3133(x: i32) -> i32 { n_5_31330(x) + n_5_31331(x) + n_5_31332(x) + n_5_31333(x) }
fn n_3_313(x: i32) -> i32 { n_4_3130(x) + n_4_3131(x) + n_4_3132(x) + n_4_3133(x) }
fn n_2_31(x: i32) -> i32 { n_3_310(x) + n_3_311(x) + n_3_312(x) + n_3_313(x) }
fn n_5_32000(x: i32) -> i32 { x.wrapping_add(1199) }
fn n_5_32001(x: i32) -> i32 { x.wrapping_add(1200) }
fn n_5_32002(x: i32) -> i32 { x.wrapping_add(1201) }
fn n_5_32003(x: i32) -> i32 { x.wrapping_add(1202) }
fn n_4_3200(x: i32) -> i32 { n_5_32000(x) + n_5_32001(x) + n_5_32002(x) + n_5_32003(x) }
fn n_5_32010(x: i32) -> i32 { x.wrapping_add(1204) }
fn n_5_32011(x: i32) -> i32 { x.wrapping_add(1205) }
fn n_5_32012(x: i32) -> i32 { x.wrapping_add(1206) }
fn n_5_32013(x: i32) -> i32 { x.wrapping_add(1207) }
fn n_4_3201(x: i32) -> i32 { n_5_32010(x) + n_5_32011(x) + n_5_32012(x) + n_5_32013(x) }
fn n_5_32020(x: i32) -> i32 { x.wrapping_add(1209) }
fn n_5_32021(x: i32) -> i32 { x.wrapping_add(1210) }
fn n_5_32022(x: i32) -> i32 { x.wrapping_add(1211) }
fn n_5_32023(x: i32) -> i32 { x.wrapping_add(1212) }
fn n_4_3202(x: i32) -> i32 { n_5_32020(x) + n_5_32021(x) + n_5_32022(x) + n_5_32023(x) }
fn n_5_32030(x: i32) -> i32 { x.wrapping_add(1214) }
fn n_5_32031(x: i32) -> i32 { x.wrapping_add(1215) }
fn n_5_32032(x: i32) -> i32 { x.wrapping_add(1216) }
fn n_5_32033(x: i32) -> i32 { x.wrapping_add(1217) }
fn n_4_3203(x: i32) -> i32 { n_5_32030(x) + n_5_32031(x) + n_5_32032(x) + n_5_32033(x) }
fn n_3_320(x: i32) -> i32 { n_4_3200(x) + n_4_3201(x) + n_4_3202(x) + n_4_3203(x) }
fn n_5_32100(x: i32) -> i32 { x.wrapping_add(1220) }
fn n_5_32101(x: i32) -> i32 { x.wrapping_add(1221) }
fn n_5_32102(x: i32) -> i32 { x.wrapping_add(1222) }
fn n_5_32103(x: i32) -> i32 { x.wrapping_add(1223) }
fn n_4_3210(x: i32) -> i32 { n_5_32100(x) + n_5_32101(x) + n_5_32102(x) + n_5_32103(x) }
fn n_5_32110(x: i32) -> i32 { x.wrapping_add(1225) }
fn n_5_32111(x: i32) -> i32 { x.wrapping_add(1226) }
fn n_5_32112(x: i32) -> i32 { x.wrapping_add(1227) }
fn n_5_32113(x: i32) -> i32 { x.wrapping_add(1228) }
fn n_4_3211(x: i32) -> i32 { n_5_32110(x) + n_5_32111(x) + n_5_32112(x) + n_5_32113(x) }
fn n_5_32120(x: i32) -> i32 { x.wrapping_add(1230) }
fn n_5_32121(x: i32) -> i32 { x.wrapping_add(1231) }
fn n_5_32122(x: i32) -> i32 { x.wrapping_add(1232) }
fn n_5_32123(x: i32) -> i32 { x.wrapping_add(1233) }
fn n_4_3212(x: i32) -> i32 { n_5_32120(x) + n_5_32121(x) + n_5_32122(x) + n_5_32123(x) }
fn n_5_32130(x: i32) -> i32 { x.wrapping_add(1235) }
fn n_5_32131(x: i32) -> i32 { x.wrapping_add(1236) }
fn n_5_32132(x: i32) -> i32 { x.wrapping_add(1237) }
fn n_5_32133(x: i32) -> i32 { x.wrapping_add(1238) }
fn n_4_3213(x: i32) -> i32 { n_5_32130(x) + n_5_32131(x) + n_5_32132(x) + n_5_32133(x) }
fn n_3_321(x: i32) -> i32 { n_4_3210(x) + n_4_3211(x) + n_4_3212(x) + n_4_3213(x) }
fn n_5_32200(x: i32) -> i32 { x.wrapping_add(1241) }
fn n_5_32201(x: i32) -> i32 { x.wrapping_add(1242) }
fn n_5_32202(x: i32) -> i32 { x.wrapping_add(1243) }
fn n_5_32203(x: i32) -> i32 { x.wrapping_add(1244) }
fn n_4_3220(x: i32) -> i32 { n_5_32200(x) + n_5_32201(x) + n_5_32202(x) + n_5_32203(x) }
fn n_5_32210(x: i32) -> i32 { x.wrapping_add(1246) }
fn n_5_32211(x: i32) -> i32 { x.wrapping_add(1247) }
fn n_5_32212(x: i32) -> i32 { x.wrapping_add(1248) }
fn n_5_32213(x: i32) -> i32 { x.wrapping_add(1249) }
fn n_4_3221(x: i32) -> i32 { n_5_32210(x) + n_5_32211(x) + n_5_32212(x) + n_5_32213(x) }
fn n_5_32220(x: i32) -> i32 { x.wrapping_add(1251) }
fn n_5_32221(x: i32) -> i32 { x.wrapping_add(1252) }
fn n_5_32222(x: i32) -> i32 { x.wrapping_add(1253) }
fn n_5_32223(x: i32) -> i32 { x.wrapping_add(1254) }
fn n_4_3222(x: i32) -> i32 { n_5_32220(x) + n_5_32221(x) + n_5_32222(x) + n_5_32223(x) }
fn n_5_32230(x: i32) -> i32 { x.wrapping_add(1256) }
fn n_5_32231(x: i32) -> i32 { x.wrapping_add(1257) }
fn n_5_32232(x: i32) -> i32 { x.wrapping_add(1258) }
fn n_5_32233(x: i32) -> i32 { x.wrapping_add(1259) }
fn n_4_3223(x: i32) -> i32 { n_5_32230(x) + n_5_32231(x) + n_5_32232(x) + n_5_32233(x) }
fn n_3_322(x: i32) -> i32 { n_4_3220(x) + n_4_3221(x) + n_4_3222(x) + n_4_3223(x) }
fn n_5_32300(x: i32) -> i32 { x.wrapping_add(1262) }
fn n_5_32301(x: i32) -> i32 { x.wrapping_add(1263) }
fn n_5_32302(x: i32) -> i32 { x.wrapping_add(1264) }
fn n_5_32303(x: i32) -> i32 { x.wrapping_add(1265) }
fn n_4_3230(x: i32) -> i32 { n_5_32300(x) + n_5_32301(x) + n_5_32302(x) + n_5_32303(x) }
fn n_5_32310(x: i32) -> i32 { x.wrapping_add(1267) }
fn n_5_32311(x: i32) -> i32 { x.wrapping_add(1268) }
fn n_5_32312(x: i32) -> i32 { x.wrapping_add(1269) }
fn n_5_32313(x: i32) -> i32 { x.wrapping_add(1270) }
fn n_4_3231(x: i32) -> i32 { n_5_32310(x) + n_5_32311(x) + n_5_32312(x) + n_5_32313(x) }
fn n_5_32320(x: i32) -> i32 { x.wrapping_add(1272) }
fn n_5_32321(x: i32) -> i32 { x.wrapping_add(1273) }
fn n_5_32322(x: i32) -> i32 { x.wrapping_add(1274) }
fn n_5_32323(x: i32) -> i32 { x.wrapping_add(1275) }
fn n_4_3232(x: i32) -> i32 { n_5_32320(x) + n_5_32321(x) + n_5_32322(x) + n_5_32323(x) }
fn n_5_32330(x: i32) -> i32 { x.wrapping_add(1277) }
fn n_5_32331(x: i32) -> i32 { x.wrapping_add(1278) }
fn n_5_32332(x: i32) -> i32 { x.wrapping_add(1279) }
fn n_5_32333(x: i32) -> i32 { x.wrapping_add(1280) }
fn n_4_3233(x: i32) -> i32 { n_5_32330(x) + n_5_32331(x) + n_5_32332(x) + n_5_32333(x) }
fn n_3_323(x: i32) -> i32 { n_4_3230(x) + n_4_3231(x) + n_4_3232(x) + n_4_3233(x) }
fn n_2_32(x: i32) -> i32 { n_3_320(x) + n_3_321(x) + n_3_322(x) + n_3_323(x) }
fn n_5_33000(x: i32) -> i32 { x.wrapping_add(1284) }
fn n_5_33001(x: i32) -> i32 { x.wrapping_add(1285) }
fn n_5_33002(x: i32) -> i32 { x.wrapping_add(1286) }
fn n_5_33003(x: i32) -> i32 { x.wrapping_add(1287) }
fn n_4_3300(x: i32) -> i32 { n_5_33000(x) + n_5_33001(x) + n_5_33002(x) + n_5_33003(x) }
fn n_5_33010(x: i32) -> i32 { x.wrapping_add(1289) }
fn n_5_33011(x: i32) -> i32 { x.wrapping_add(1290) }
fn n_5_33012(x: i32) -> i32 { x.wrapping_add(1291) }
fn n_5_33013(x: i32) -> i32 { x.wrapping_add(1292) }
fn n_4_3301(x: i32) -> i32 { n_5_33010(x) + n_5_33011(x) + n_5_33012(x) + n_5_33013(x) }
fn n_5_33020(x: i32) -> i32 { x.wrapping_add(1294) }
fn n_5_33021(x: i32) -> i32 { x.wrapping_add(1295) }
fn n_5_33022(x: i32) -> i32 { x.wrapping_add(1296) }
fn n_5_33023(x: i32) -> i32 { x.wrapping_add(1297) }
fn n_4_3302(x: i32) -> i32 { n_5_33020(x) + n_5_33021(x) + n_5_33022(x) + n_5_33023(x) }
fn n_5_33030(x: i32) -> i32 { x.wrapping_add(1299) }
fn n_5_33031(x: i32) -> i32 { x.wrapping_add(1300) }
fn n_5_33032(x: i32) -> i32 { x.wrapping_add(1301) }
fn n_5_33033(x: i32) -> i32 { x.wrapping_add(1302) }
fn n_4_3303(x: i32) -> i32 { n_5_33030(x) + n_5_33031(x) + n_5_33032(x) + n_5_33033(x) }
fn n_3_330(x: i32) -> i32 { n_4_3300(x) + n_4_3301(x) + n_4_3302(x) + n_4_3303(x) }
fn n_5_33100(x: i32) -> i32 { x.wrapping_add(1305) }
fn n_5_33101(x: i32) -> i32 { x.wrapping_add(1306) }
fn n_5_33102(x: i32) -> i32 { x.wrapping_add(1307) }
fn n_5_33103(x: i32) -> i32 { x.wrapping_add(1308) }
fn n_4_3310(x: i32) -> i32 { n_5_33100(x) + n_5_33101(x) + n_5_33102(x) + n_5_33103(x) }
fn n_5_33110(x: i32) -> i32 { x.wrapping_add(1310) }
fn n_5_33111(x: i32) -> i32 { x.wrapping_add(1311) }
fn n_5_33112(x: i32) -> i32 { x.wrapping_add(1312) }
fn n_5_33113(x: i32) -> i32 { x.wrapping_add(1313) }
fn n_4_3311(x: i32) -> i32 { n_5_33110(x) + n_5_33111(x) + n_5_33112(x) + n_5_33113(x) }
fn n_5_33120(x: i32) -> i32 { x.wrapping_add(1315) }
fn n_5_33121(x: i32) -> i32 { x.wrapping_add(1316) }
fn n_5_33122(x: i32) -> i32 { x.wrapping_add(1317) }
fn n_5_33123(x: i32) -> i32 { x.wrapping_add(1318) }
fn n_4_3312(x: i32) -> i32 { n_5_33120(x) + n_5_33121(x) + n_5_33122(x) + n_5_33123(x) }
fn n_5_33130(x: i32) -> i32 { x.wrapping_add(1320) }
fn n_5_33131(x: i32) -> i32 { x.wrapping_add(1321) }
fn n_5_33132(x: i32) -> i32 { x.wrapping_add(1322) }
fn n_5_33133(x: i32) -> i32 { x.wrapping_add(1323) }
fn n_4_3313(x: i32) -> i32 { n_5_33130(x) + n_5_33131(x) + n_5_33132(x) + n_5_33133(x) }
fn n_3_331(x: i32) -> i32 { n_4_3310(x) + n_4_3311(x) + n_4_3312(x) + n_4_3313(x) }
fn n_5_33200(x: i32) -> i32 { x.wrapping_add(1326) }
fn n_5_33201(x: i32) -> i32 { x.wrapping_add(1327) }
fn n_5_33202(x: i32) -> i32 { x.wrapping_add(1328) }
fn n_5_33203(x: i32) -> i32 { x.wrapping_add(1329) }
fn n_4_3320(x: i32) -> i32 { n_5_33200(x) + n_5_33201(x) + n_5_33202(x) + n_5_33203(x) }
fn n_5_33210(x: i32) -> i32 { x.wrapping_add(1331) }
fn n_5_33211(x: i32) -> i32 { x.wrapping_add(1332) }
fn n_5_33212(x: i32) -> i32 { x.wrapping_add(1333) }
fn n_5_33213(x: i32) -> i32 { x.wrapping_add(1334) }
fn n_4_3321(x: i32) -> i32 { n_5_33210(x) + n_5_33211(x) + n_5_33212(x) + n_5_33213(x) }
fn n_5_33220(x: i32) -> i32 { x.wrapping_add(1336) }
fn n_5_33221(x: i32) -> i32 { x.wrapping_add(1337) }
fn n_5_33222(x: i32) -> i32 { x.wrapping_add(1338) }
fn n_5_33223(x: i32) -> i32 { x.wrapping_add(1339) }
fn n_4_3322(x: i32) -> i32 { n_5_33220(x) + n_5_33221(x) + n_5_33222(x) + n_5_33223(x) }
fn n_5_33230(x: i32) -> i32 { x.wrapping_add(1341) }
fn n_5_33231(x: i32) -> i32 { x.wrapping_add(1342) }
fn n_5_33232(x: i32) -> i32 { x.wrapping_add(1343) }
fn n_5_33233(x: i32) -> i32 { x.wrapping_add(1344) }
fn n_4_3323(x: i32) -> i32 { n_5_33230(x) + n_5_33231(x) + n_5_33232(x) + n_5_33233(x) }
fn n_3_332(x: i32) -> i32 { n_4_3320(x) + n_4_3321(x) + n_4_3322(x) + n_4_3323(x) }
fn n_5_33300(x: i32) -> i32 { x.wrapping_add(1347) }
fn n_5_33301(x: i32) -> i32 { x.wrapping_add(1348) }
fn n_5_33302(x: i32) -> i32 { x.wrapping_add(1349) }
fn n_5_33303(x: i32) -> i32 { x.wrapping_add(1350) }
fn n_4_3330(x: i32) -> i32 { n_5_33300(x) + n_5_33301(x) + n_5_33302(x) + n_5_33303(x) }
fn n_5_33310(x: i32) -> i32 { x.wrapping_add(1352) }
fn n_5_33311(x: i32) -> i32 { x.wrapping_add(1353) }
fn n_5_33312(x: i32) -> i32 { x.wrapping_add(1354) }
fn n_5_33313(x: i32) -> i32 { x.wrapping_add(1355) }
fn n_4_3331(x: i32) -> i32 { n_5_33310(x) + n_5_33311(x) + n_5_33312(x) + n_5_33313(x) }
fn n_5_33320(x: i32) -> i32 { x.wrapping_add(1357) }
fn n_5_33321(x: i32) -> i32 { x.wrapping_add(1358) }
fn n_5_33322(x: i32) -> i32 { x.wrapping_add(1359) }
fn n_5_33323(x: i32) -> i32 { x.wrapping_add(1360) }
fn n_4_3332(x: i32) -> i32 { n_5_33320(x) + n_5_33321(x) + n_5_33322(x) + n_5_33323(x) }
fn n_5_33330(x: i32) -> i32 { x.wrapping_add(1362) }
fn n_5_33331(x: i32) -> i32 { x.wrapping_add(1363) }
fn n_5_33332(x: i32) -> i32 { x.wrapping_add(1364) }
fn n_5_33333(x: i32) -> i32 { x.wrapping_add(1365) }
fn n_4_3333(x: i32) -> i32 { n_5_33330(x) + n_5_33331(x) + n_5_33332(x) + n_5_33333(x) }
fn n_3_333(x: i32) -> i32 { n_4_3330(x) + n_4_3331(x) + n_4_3332(x) + n_4_3333(x) }
fn n_2_33(x: i32) -> i32 { n_3_330(x) + n_3_331(x) + n_3_332(x) + n_3_333(x) }
fn n_1_3(x: i32) -> i32 { n_2_30(x) + n_2_31(x) + n_2_32(x) + n_2_33(x) }
fn n_0_(x: i32) -> i32 { n_1_0(x) + n_1_1(x) + n_1_2(x) + n_1_3(x) }

#[no_alloc_check::no_alloc]
fn root() -> i32 {
    n_0_(41)
}

fn main() {
    println!("{}", root());
}
