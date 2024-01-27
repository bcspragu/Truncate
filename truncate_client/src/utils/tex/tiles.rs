#![cfg_attr(rustfmt, rustfmt_skip)]

#![allow(dead_code)]

use super::{Tex, TexQuad};

const fn t(tile: usize) -> Tex {
    Tex { tile, tint: None }
}

pub const MAX_TILE: usize = 205;
pub const NONE: Tex = t(0);
pub const DEBUG: Tex = t(1);
pub const BASE_GRASS: Tex = t(2);
pub const GRASS_0_WIND_0: Tex = t(3);
pub const GRASS_0_WIND_1: Tex = t(4);
pub const GRASS_0_WIND_2: Tex = t(5);
pub const GRASS_0_WIND_3: Tex = t(6);
pub const GRASS_1_WIND_0: Tex = t(7);
pub const GRASS_1_WIND_1: Tex = t(8);
pub const GRASS_1_WIND_2: Tex = t(9);
pub const GRASS_1_WIND_3: Tex = t(10);
pub const GRASS_2_WIND_0: Tex = t(11);
pub const GRASS_2_WIND_1: Tex = t(12);
pub const GRASS_2_WIND_2: Tex = t(13);
pub const GRASS_2_WIND_3: Tex = t(14);
pub const CHECKERBOARD_NW: Tex = t(15);
pub const CHECKERBOARD_NE: Tex = t(16);
pub const CHECKERBOARD_SW: Tex = t(17);
pub const CHECKERBOARD_SE: Tex = t(18);
pub const BASE_WATER: Tex = t(19);
pub const WATER_WITH_LAND_SE: Tex = t(20);
pub const WATER_WITH_LAND_S: Tex = t(21);
pub const WATER_WITH_LAND_SW: Tex = t(22);
pub const WATER_WITH_LAND_E: Tex = t(23);
pub const WATER_WITH_LAND_W: Tex = t(24);
pub const WATER_WITH_LAND_NE: Tex = t(25);
pub const WATER_WITH_LAND_N: Tex = t(26);
pub const WATER_WITH_LAND_NW: Tex = t(27);
pub const LAND_WITH_WATER_SE: Tex = t(28);
pub const LAND_WITH_WATER_SW: Tex = t(29);
pub const LAND_WITH_WATER_NE: Tex = t(30);
pub const LAND_WITH_WATER_NW: Tex = t(31);
pub const ISLAND_NW: Tex = t(32);
pub const ISLAND_NE: Tex = t(33);
pub const ISLAND_SW: Tex = t(34);
pub const ISLAND_SE: Tex = t(35);
pub const LAKE_NW: Tex = t(36);
pub const LAKE_NE: Tex = t(37);
pub const LAKE_SW: Tex = t(38);
pub const LAKE_SE: Tex = t(39);
pub const MISC_COBBLE_0: Tex = t(40);
pub const MISC_COBBLE_1: Tex = t(41);
pub const MISC_COBBLE_2: Tex = t(42);
pub const MISC_COBBLE_3: Tex = t(43);
pub const MISC_COBBLE_4: Tex = t(44);
pub const MISC_COBBLE_5: Tex = t(45);
pub const MISC_COBBLE_6: Tex = t(46);
pub const MISC_COBBLE_7: Tex = t(47);
pub const MISC_COBBLE_8: Tex = t(48);
pub const HOUSE_0: Tex = t(49);
pub const ROOF_0: Tex = t(50);
pub const HOUSE_1: Tex = t(51);
pub const ROOF_1: Tex = t(52);
pub const HOUSE_2: Tex = t(53);
pub const ROOF_2: Tex = t(54);
pub const HOUSE_3: Tex = t(55);
pub const ROOF_3: Tex = t(56);
pub const SMOKE_0: Tex = t(57);
pub const SMOKE_1: Tex = t(58);
pub const SMOKE_2: Tex = t(59);
pub const SMOKE_3: Tex = t(60);
pub const SMOKE_4: Tex = t(61);
pub const BUSH_0: Tex = t(62);
pub const BUSH_FLOWERS_0: Tex = t(63);
pub const BUSH_1: Tex = t(64);
pub const BUSH_FLOWERS_1: Tex = t(65);
pub const BUSH_2: Tex = t(66);
pub const BUSH_FLOWERS_2: Tex = t(67);
pub const WHEAT_WIND_0: Tex = t(68);
pub const WHEAT_WIND_1: Tex = t(69);
pub const WELL: Tex = t(70);
pub const NORTH_DOCK_NW: Tex = t(71);
pub const NORTH_DOCK_NE: Tex = t(72);
pub const NORTH_DOCK_SW: Tex = t(73);
pub const NORTH_DOCK_SE: Tex = t(74);
pub const NORTH_DOCK_SAIL_WIND_0_NW: Tex = t(75);
pub const NORTH_DOCK_SAIL_WIND_0_NE: Tex = t(76);
pub const NORTH_DOCK_SAIL_WIND_1_NW: Tex = t(77);
pub const NORTH_DOCK_SAIL_WIND_1_NE: Tex = t(78);
pub const NORTH_DOCK_SAIL_WIND_2_NW: Tex = t(79);
pub const NORTH_DOCK_SAIL_WIND_2_NE: Tex = t(80);
pub const SOUTH_DOCK_NW: Tex = t(81);
pub const SOUTH_DOCK_NE: Tex = t(82);
pub const SOUTH_DOCK_SW: Tex = t(83);
pub const SOUTH_DOCK_SE: Tex = t(84);
pub const SOUTH_DOCK_SAIL_WIND_0_NW: Tex = t(85);
pub const SOUTH_DOCK_SAIL_WIND_0_SW: Tex = t(86);
pub const SOUTH_DOCK_SAIL_WIND_1_NW: Tex = t(87);
pub const SOUTH_DOCK_SAIL_WIND_1_SW: Tex = t(88);
pub const SOUTH_DOCK_SAIL_WIND_2_NW: Tex = t(89);
pub const SOUTH_DOCK_SAIL_WIND_2_SW: Tex = t(90);
pub const EAST_DOCK_NW: Tex = t(91);
pub const EAST_DOCK_NE: Tex = t(92);
pub const EAST_DOCK_SW: Tex = t(93);
pub const EAST_DOCK_SE: Tex = t(94);
pub const EAST_DOCK_SAIL_WIND_0_NW: Tex = t(95);
pub const EAST_DOCK_SAIL_WIND_0_NE: Tex = t(96);
pub const EAST_DOCK_SAIL_WIND_0_SW: Tex = t(97);
pub const EAST_DOCK_SAIL_WIND_0_SE: Tex = t(98);
pub const EAST_DOCK_SAIL_WIND_1_NW: Tex = t(99);
pub const EAST_DOCK_SAIL_WIND_1_NE: Tex = t(100);
pub const EAST_DOCK_SAIL_WIND_1_SW: Tex = t(101);
pub const EAST_DOCK_SAIL_WIND_1_SE: Tex = t(102);
pub const EAST_DOCK_SAIL_WIND_2_NW: Tex = t(103);
pub const EAST_DOCK_SAIL_WIND_2_NE: Tex = t(104);
pub const EAST_DOCK_SAIL_WIND_2_SW: Tex = t(105);
pub const EAST_DOCK_SAIL_WIND_2_SE: Tex = t(106);
pub const WEST_DOCK_NW: Tex = t(107);
pub const WEST_DOCK_NE: Tex = t(108);
pub const WEST_DOCK_SW: Tex = t(109);
pub const WEST_DOCK_SE: Tex = t(110);
pub const WEST_DOCK_SAIL_WIND_0_NW: Tex = t(111);
pub const WEST_DOCK_SAIL_WIND_0_NE: Tex = t(112);
pub const WEST_DOCK_SAIL_WIND_1_NW: Tex = t(113);
pub const WEST_DOCK_SAIL_WIND_1_NE: Tex = t(114);
pub const WEST_DOCK_SAIL_WIND_2_NW: Tex = t(115);
pub const WEST_DOCK_SAIL_WIND_2_NE: Tex = t(116);
pub const FLOATING_DOCK_NW: Tex = t(117);
pub const FLOATING_DOCK_NE: Tex = t(118);
pub const FLOATING_DOCK_SW: Tex = t(119);
pub const FLOATING_DOCK_SE: Tex = t(120);
pub const FLOATING_DOCK_SAIL_WIND_0_NW: Tex = t(121);
pub const FLOATING_DOCK_SAIL_WIND_0_NE: Tex = t(122);
pub const FLOATING_DOCK_SAIL_WIND_1_NW: Tex = t(123);
pub const FLOATING_DOCK_SAIL_WIND_1_NE: Tex = t(124);
pub const FLOATING_DOCK_SAIL_WIND_2_NW: Tex = t(125);
pub const FLOATING_DOCK_SAIL_WIND_2_NE: Tex = t(126);
pub const GAME_PIECE_NW: Tex = t(127);
pub const GAME_PIECE_NE: Tex = t(128);
pub const GAME_PIECE_SW: Tex = t(129);
pub const GAME_PIECE_SE: Tex = t(130);
pub const HIGHLIGHT_NW: Tex = t(131);
pub const HIGHLIGHT_NE: Tex = t(132);
pub const HIGHLIGHT_SW: Tex = t(133);
pub const HIGHLIGHT_SE: Tex = t(134);
pub const GAME_PIECE_N: Tex = t(135);
pub const GAME_PIECE_S: Tex = t(136);
pub const GAME_PIECE_GRASS_0_SW: Tex = t(137);
pub const GAME_PIECE_GRASS_0_SE: Tex = t(138);
pub const GAME_PIECE_GRASS_1_SW: Tex = t(139);
pub const GAME_PIECE_GRASS_1_SE: Tex = t(140);
pub const GAME_PIECE_GRASS_2_SW: Tex = t(141);
pub const GAME_PIECE_GRASS_2_SE: Tex = t(142);
pub const GAME_PIECE_GRASS_3_SW: Tex = t(143);
pub const GAME_PIECE_GRASS_3_SE: Tex = t(144);
pub const GAME_PIECE_CRACKS_0_SW: Tex = t(145);
pub const GAME_PIECE_CRACKS_0_SE: Tex = t(146);
pub const GAME_PIECE_CRACKS_1_NW: Tex = t(147);
pub const GAME_PIECE_CRACKS_1_NE: Tex = t(148);
pub const GAME_PIECE_CRACKS_1_SE: Tex = t(149);
pub const GAME_PIECE_CRACKS_2_NE: Tex = t(150);
pub const GAME_PIECE_CRACKS_2_SE: Tex = t(151);
pub const GAME_PIECE_CRACKS_3_NW: Tex = t(152);
pub const GAME_PIECE_CRACKS_3_SW: Tex = t(153);
pub const GAME_PIECE_CRACKS_4_NW: Tex = t(154);
pub const GAME_PIECE_RUBBLE_0_NW: Tex = t(155);
pub const GAME_PIECE_RUBBLE_0_NE: Tex = t(156);
pub const GAME_PIECE_RUBBLE_0_SW: Tex = t(157);
pub const GAME_PIECE_RUBBLE_0_SE: Tex = t(158);
pub const GAME_PIECE_RUBBLE_1_NW: Tex = t(159);
pub const GAME_PIECE_RUBBLE_1_NE: Tex = t(160);
pub const GAME_PIECE_RUBBLE_1_SW: Tex = t(161);
pub const GAME_PIECE_RUBBLE_1_SE: Tex = t(162);
pub const GAME_PIECE_RUBBLE_2_NW: Tex = t(163);
pub const GAME_PIECE_RUBBLE_2_NE: Tex = t(164);
pub const GAME_PIECE_RUBBLE_2_SW: Tex = t(165);
pub const GAME_PIECE_RUBBLE_2_SE: Tex = t(166);
pub const DIALOG_NW: Tex = t(167);
pub const DIALOG_N: Tex = t(168);
pub const DIALOG_NE: Tex = t(169);
pub const DIALOG_W: Tex = t(170);
pub const DIALOG_CENTER: Tex = t(171);
pub const DIALOG_E: Tex = t(172);
pub const DIALOG_SW: Tex = t(173);
pub const DIALOG_S: Tex = t(174);
pub const DIALOG_SE: Tex = t(175);
pub const INFO_BUTTON_NW: Tex = t(176);
pub const INFO_BUTTON_NE: Tex = t(177);
pub const INFO_BUTTON_SW: Tex = t(178);
pub const INFO_BUTTON_SE: Tex = t(179);
pub const CLOSE_BUTTON_NW: Tex = t(180);
pub const CLOSE_BUTTON_NE: Tex = t(181);
pub const CLOSE_BUTTON_SW: Tex = t(182);
pub const CLOSE_BUTTON_SE: Tex = t(183);
pub const BUTTON_NOTIFICATION_SE: Tex = t(184);
pub const BARE_CLOSE_BUTTON_NW: Tex = t(185);
pub const BARE_CLOSE_BUTTON_NE: Tex = t(186);
pub const BARE_CLOSE_BUTTON_SW: Tex = t(187);
pub const BARE_CLOSE_BUTTON_SE: Tex = t(188);
pub const TOWN_BUTTON_NW: Tex = t(189);
pub const TOWN_BUTTON_NE: Tex = t(190);
pub const TOWN_BUTTON_SW: Tex = t(191);
pub const TOWN_BUTTON_SE: Tex = t(192);
pub const TOWN_BUTTON_ROOF_NW: Tex = t(193);
pub const TOWN_BUTTON_ROOF_NE: Tex = t(194);
pub const DOCK_BUTTON_NW: Tex = t(195);
pub const DOCK_BUTTON_NE: Tex = t(196);
pub const DOCK_BUTTON_SW: Tex = t(197);
pub const DOCK_BUTTON_SE: Tex = t(198);
pub const DOCK_BUTTON_SAIL_NW: Tex = t(199);
pub const DOCK_BUTTON_SAIL_NE: Tex = t(200);
pub const TERRAIN_BUTTON_NW: Tex = t(201);
pub const TERRAIN_BUTTON_NE: Tex = t(202);
pub const TERRAIN_BUTTON_SW: Tex = t(203);
pub const TERRAIN_BUTTON_SE: Tex = t(204);

pub mod quad {
    use super::*;

    pub const CHECKERBOARD: TexQuad = [CHECKERBOARD_NW, CHECKERBOARD_NE, CHECKERBOARD_SE, CHECKERBOARD_SW];
    pub const WATER_WITH_LAND: TexQuad = [WATER_WITH_LAND_NW, WATER_WITH_LAND_NE, WATER_WITH_LAND_SE, WATER_WITH_LAND_SW];
    pub const LAND_WITH_WATER: TexQuad = [LAND_WITH_WATER_NW, LAND_WITH_WATER_NE, LAND_WITH_WATER_SE, LAND_WITH_WATER_SW];
    pub const ISLAND: TexQuad = [ISLAND_NW, ISLAND_NE, ISLAND_SE, ISLAND_SW];
    pub const LAKE: TexQuad = [LAKE_NW, LAKE_NE, LAKE_SE, LAKE_SW];
    pub const NORTH_DOCK: TexQuad = [NORTH_DOCK_NW, NORTH_DOCK_NE, NORTH_DOCK_SE, NORTH_DOCK_SW];
    pub const NORTH_DOCK_SAIL_WIND_0: TexQuad = [NORTH_DOCK_SAIL_WIND_0_NW, NORTH_DOCK_SAIL_WIND_0_NE, NONE, NONE];
    pub const NORTH_DOCK_SAIL_WIND_1: TexQuad = [NORTH_DOCK_SAIL_WIND_1_NW, NORTH_DOCK_SAIL_WIND_1_NE, NONE, NONE];
    pub const NORTH_DOCK_SAIL_WIND_2: TexQuad = [NORTH_DOCK_SAIL_WIND_2_NW, NORTH_DOCK_SAIL_WIND_2_NE, NONE, NONE];
    pub const SOUTH_DOCK: TexQuad = [SOUTH_DOCK_NW, SOUTH_DOCK_NE, SOUTH_DOCK_SE, SOUTH_DOCK_SW];
    pub const SOUTH_DOCK_SAIL_WIND_0: TexQuad = [SOUTH_DOCK_SAIL_WIND_0_NW, NONE, NONE, SOUTH_DOCK_SAIL_WIND_0_SW];
    pub const SOUTH_DOCK_SAIL_WIND_1: TexQuad = [SOUTH_DOCK_SAIL_WIND_1_NW, NONE, NONE, SOUTH_DOCK_SAIL_WIND_1_SW];
    pub const SOUTH_DOCK_SAIL_WIND_2: TexQuad = [SOUTH_DOCK_SAIL_WIND_2_NW, NONE, NONE, SOUTH_DOCK_SAIL_WIND_2_SW];
    pub const EAST_DOCK: TexQuad = [EAST_DOCK_NW, EAST_DOCK_NE, EAST_DOCK_SE, EAST_DOCK_SW];
    pub const EAST_DOCK_SAIL_WIND_0: TexQuad = [EAST_DOCK_SAIL_WIND_0_NW, EAST_DOCK_SAIL_WIND_0_NE, EAST_DOCK_SAIL_WIND_0_SE, EAST_DOCK_SAIL_WIND_0_SW];
    pub const EAST_DOCK_SAIL_WIND_1: TexQuad = [EAST_DOCK_SAIL_WIND_1_NW, EAST_DOCK_SAIL_WIND_1_NE, EAST_DOCK_SAIL_WIND_1_SE, EAST_DOCK_SAIL_WIND_1_SW];
    pub const EAST_DOCK_SAIL_WIND_2: TexQuad = [EAST_DOCK_SAIL_WIND_2_NW, EAST_DOCK_SAIL_WIND_2_NE, EAST_DOCK_SAIL_WIND_2_SE, EAST_DOCK_SAIL_WIND_2_SW];
    pub const WEST_DOCK: TexQuad = [WEST_DOCK_NW, WEST_DOCK_NE, WEST_DOCK_SE, WEST_DOCK_SW];
    pub const WEST_DOCK_SAIL_WIND_0: TexQuad = [WEST_DOCK_SAIL_WIND_0_NW, WEST_DOCK_SAIL_WIND_0_NE, NONE, NONE];
    pub const WEST_DOCK_SAIL_WIND_1: TexQuad = [WEST_DOCK_SAIL_WIND_1_NW, WEST_DOCK_SAIL_WIND_1_NE, NONE, NONE];
    pub const WEST_DOCK_SAIL_WIND_2: TexQuad = [WEST_DOCK_SAIL_WIND_2_NW, WEST_DOCK_SAIL_WIND_2_NE, NONE, NONE];
    pub const FLOATING_DOCK: TexQuad = [FLOATING_DOCK_NW, FLOATING_DOCK_NE, FLOATING_DOCK_SE, FLOATING_DOCK_SW];
    pub const FLOATING_DOCK_SAIL_WIND_0: TexQuad = [FLOATING_DOCK_SAIL_WIND_0_NW, FLOATING_DOCK_SAIL_WIND_0_NE, NONE, NONE];
    pub const FLOATING_DOCK_SAIL_WIND_1: TexQuad = [FLOATING_DOCK_SAIL_WIND_1_NW, FLOATING_DOCK_SAIL_WIND_1_NE, NONE, NONE];
    pub const FLOATING_DOCK_SAIL_WIND_2: TexQuad = [FLOATING_DOCK_SAIL_WIND_2_NW, FLOATING_DOCK_SAIL_WIND_2_NE, NONE, NONE];
    pub const GAME_PIECE: TexQuad = [GAME_PIECE_NW, GAME_PIECE_NE, GAME_PIECE_SE, GAME_PIECE_SW];
    pub const HIGHLIGHT: TexQuad = [HIGHLIGHT_NW, HIGHLIGHT_NE, HIGHLIGHT_SE, HIGHLIGHT_SW];
    pub const GAME_PIECE_GRASS_0: TexQuad = [NONE, NONE, GAME_PIECE_GRASS_0_SE, GAME_PIECE_GRASS_0_SW];
    pub const GAME_PIECE_GRASS_1: TexQuad = [NONE, NONE, GAME_PIECE_GRASS_1_SE, GAME_PIECE_GRASS_1_SW];
    pub const GAME_PIECE_GRASS_2: TexQuad = [NONE, NONE, GAME_PIECE_GRASS_2_SE, GAME_PIECE_GRASS_2_SW];
    pub const GAME_PIECE_GRASS_3: TexQuad = [NONE, NONE, GAME_PIECE_GRASS_3_SE, GAME_PIECE_GRASS_3_SW];
    pub const GAME_PIECE_CRACKS_0: TexQuad = [NONE, NONE, GAME_PIECE_CRACKS_0_SE, GAME_PIECE_CRACKS_0_SW];
    pub const GAME_PIECE_CRACKS_1: TexQuad = [GAME_PIECE_CRACKS_1_NW, GAME_PIECE_CRACKS_1_NE, GAME_PIECE_CRACKS_1_SE, NONE];
    pub const GAME_PIECE_CRACKS_2: TexQuad = [NONE, GAME_PIECE_CRACKS_2_NE, GAME_PIECE_CRACKS_2_SE, NONE];
    pub const GAME_PIECE_CRACKS_3: TexQuad = [GAME_PIECE_CRACKS_3_NW, NONE, NONE, GAME_PIECE_CRACKS_3_SW];
    pub const GAME_PIECE_CRACKS_4: TexQuad = [GAME_PIECE_CRACKS_4_NW, NONE, NONE, NONE];
    pub const GAME_PIECE_RUBBLE_0: TexQuad = [GAME_PIECE_RUBBLE_0_NW, GAME_PIECE_RUBBLE_0_NE, GAME_PIECE_RUBBLE_0_SE, GAME_PIECE_RUBBLE_0_SW];
    pub const GAME_PIECE_RUBBLE_1: TexQuad = [GAME_PIECE_RUBBLE_1_NW, GAME_PIECE_RUBBLE_1_NE, GAME_PIECE_RUBBLE_1_SE, GAME_PIECE_RUBBLE_1_SW];
    pub const GAME_PIECE_RUBBLE_2: TexQuad = [GAME_PIECE_RUBBLE_2_NW, GAME_PIECE_RUBBLE_2_NE, GAME_PIECE_RUBBLE_2_SE, GAME_PIECE_RUBBLE_2_SW];
    pub const DIALOG: TexQuad = [DIALOG_NW, DIALOG_NE, DIALOG_SE, DIALOG_SW];
    pub const INFO_BUTTON: TexQuad = [INFO_BUTTON_NW, INFO_BUTTON_NE, INFO_BUTTON_SE, INFO_BUTTON_SW];
    pub const CLOSE_BUTTON: TexQuad = [CLOSE_BUTTON_NW, CLOSE_BUTTON_NE, CLOSE_BUTTON_SE, CLOSE_BUTTON_SW];
    pub const BUTTON_NOTIFICATION: TexQuad = [NONE, NONE, BUTTON_NOTIFICATION_SE, NONE];
    pub const BARE_CLOSE_BUTTON: TexQuad = [BARE_CLOSE_BUTTON_NW, BARE_CLOSE_BUTTON_NE, BARE_CLOSE_BUTTON_SE, BARE_CLOSE_BUTTON_SW];
    pub const TOWN_BUTTON: TexQuad = [TOWN_BUTTON_NW, TOWN_BUTTON_NE, TOWN_BUTTON_SE, TOWN_BUTTON_SW];
    pub const TOWN_BUTTON_ROOF: TexQuad = [TOWN_BUTTON_ROOF_NW, TOWN_BUTTON_ROOF_NE, NONE, NONE];
    pub const DOCK_BUTTON: TexQuad = [DOCK_BUTTON_NW, DOCK_BUTTON_NE, DOCK_BUTTON_SE, DOCK_BUTTON_SW];
    pub const DOCK_BUTTON_SAIL: TexQuad = [DOCK_BUTTON_SAIL_NW, DOCK_BUTTON_SAIL_NE, NONE, NONE];
    pub const TERRAIN_BUTTON: TexQuad = [TERRAIN_BUTTON_NW, TERRAIN_BUTTON_NE, TERRAIN_BUTTON_SE, TERRAIN_BUTTON_SW];
}