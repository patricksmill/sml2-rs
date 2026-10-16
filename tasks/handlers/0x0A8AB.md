# Handler 0x0A8AB

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x0A8AB`, bank `$02` (2 decimal), CPU `$68AB`.
**Selected bank at entry:** `$02`.
**Anims:** boss:20.
**Policy:** [PORTING.md](../../PORTING.md).

Implement in a focused sibling module under `crates/sml2-core/src/objects/`;
register it in `objects/mod.rs` if new, and wire `objects/dispatch.rs`.
Reuse `objects/staging.rs` and `objects/helpers.rs`. New tests belong beside the
implementation; shared integration fixtures are currently in `objects/tests.rs`.

## Exact ROM-addressed instructions and ASM locations

Addresses and edges derive from ROM bytes. Source labels are not assumed to
mark every entry. Review table bounds and dependency contracts before porting.

```text
0x09285  F0 E7    main.asm:3803 ldh a, [$FFE7]
0x09287  CB 47    main.asm:3804 bit 0, a
0x09289  20 1A    main.asm:3805 jr nz, UnknownRJump_0x92A5
0x0928B  CB 4F    main.asm:3806 bit 1, a
0x0928D  20 01    main.asm:3807 jr nz, UnknownRJump_0x9290
0x0928F  C9       main.asm:3808 ret
0x09290  3E 01    main.asm:3811 ld a, 1
0x09292  EA 3E AF main.asm:3812 ld [sObjectDone], a
0x09295  FA 29 AF main.asm:3813 ld a, [sObjectSlotPtrLo]
0x09298  A7       main.asm:3814 and a
0x09299  C0       main.asm:3815 ret nz
0x0929A  3E 09    main.asm:3817 ld a, 9
0x0929C  EA 70 A4 main.asm:3818 ld [sSfxRequest1], a
0x0929F  FA C5 AF main.asm:3819 ld a, [sBossHits]
0x092A2  3D       main.asm:3820 dec a
0x092A3  18 17    main.asm:3821 jr UnknownRJump_0x92BC
0x092A5  3E 01    main.asm:3825 ld a, 1
0x092A7  EA 3E AF main.asm:3826 ld [sObjectDone], a
0x092AA  EA C3 AF main.asm:3827 ld [sBossHitLatch], a
0x092AD  FA 29 AF main.asm:3828 ld a, [sObjectSlotPtrLo]
0x092B0  A7       main.asm:3829 and a
0x092B1  C0       main.asm:3830 ret nz
0x092B2  3E 13    main.asm:3831 ld a, 19
0x092B4  EA 60 A4 main.asm:3832 ld [sSfxRequest0], a
0x092B7  FA C5 AF main.asm:3833 ld a, [sBossHits]
0x092BA  D6 02    main.asm:3834 sub 2
0x092BC  EA C5 AF main.asm:3837 ld [sBossHits], a
0x092BF  28 03    main.asm:3838 jr z, UnknownRJump_0x92C4
0x092C1  38 01    main.asm:3839 jr c, UnknownRJump_0x92C4
0x092C3  C9       main.asm:3840 ret
0x092C4  3E 01    main.asm:3843 ld a, 1
0x092C6  EA 26 AF main.asm:3844 ld [sObjectRemoveMode], a
0x092C9  FA C0 AF main.asm:3845 ld a, [sObjectDefIndex]
0x092CC  FE 06    main.asm:3846 cp $06
0x092CE  D0       main.asm:3847 ret nc
0x092CF  3E 02    main.asm:3848 ld a, 2
0x092D1  EA 67 A2 main.asm:3849 ld [sFadeTimerHigh], a
0x092D4  3E 80    main.asm:3850 ld a, 128
0x092D6  EA 66 A2 main.asm:3851 ld [sFadeTimerLow], a
0x092D9  3E 01    main.asm:3852 ld a, 1
0x092DB  EA E3 A2 main.asm:3853 ld [sBossClearFlag], a
0x092DE  C9       main.asm:3854 ret
0x09435  3E 02    main.asm:4029 ld a, 2
0x09437  E0 DF    main.asm:4030 ldh [$FFDF], a
0x09439  3E 20    main.asm:4031 ld a, 32
0x0943B  E0 EC    main.asm:4032 ldh [$FFEC], a
0x0943D  FA CA AF main.asm:4033 ld a, [sObjectDefSprite]
0x09440  E0 DB    main.asm:4034 ldh [$FFDB], a
0x09442  C9       main.asm:4035 ret
0x09725  AF       main.asm:4395 xor a
0x09726  E0 E2    main.asm:4396 ldh [$FFE2], a
0x09728  3E 01    main.asm:4397 ld a, 1
0x0972A  E0 DF    main.asm:4398 ldh [$FFDF], a
0x0972C  C9       main.asm:4399 ret
0x09CAC  CD 0E 74 main.asm:4967 call UnknownCall_0xB40E
0x09CAF  F0 EC    main.asm:4968 ldh a, [$FFEC]
0x09CB1  A7       main.asm:4969 and a
0x09CB2  28 06    main.asm:4970 jr z, UnknownRJump_0x9CBA
0x09CB4  F0 EC    main.asm:4971 ldh a, [$FFEC]
0x09CB6  3D       main.asm:4972 dec a
0x09CB7  E0 EC    main.asm:4973 ldh [$FFEC], a
0x09CB9  C0       main.asm:4974 ret nz
0x09CBA  FA 26 AF main.asm:4977 ld a, [sObjectRemoveMode]
0x09CBD  A7       main.asm:4978 and a
0x09CBE  C2 35 54 main.asm:4979 jp nz, UnknownJump_0x9435
0x09CC1  AF       main.asm:4980 xor a
0x09CC2  E0 DE    main.asm:4981 ldh [$FFDE], a
0x09CC4  FA 37 AF main.asm:4982 ld a, [sObjectSpawnSprite]
0x09CC7  E0 DB    main.asm:4983 ldh [$FFDB], a
0x09CC9  FA C4 AF main.asm:4984 ld a, [sSavedFrameState]
0x09CCC  E0 DF    main.asm:4985 ldh [$FFDF], a
0x09CCE  FA CC AF main.asm:4986 ld a, [sSavedObjectType]
0x09CD1  E0 E2    main.asm:4987 ldh [$FFE2], a
0x09CD3  C9       main.asm:4988 ret
0x0A8AB  CD 85 52 main.asm:6623 call UnknownCall_0x9285
0x0A8AE  FA 3E AF main.asm:6624 ld a, [sObjectDone]
0x0A8B1  A7       main.asm:6625 and a
0x0A8B2  28 22    main.asm:6626 jr z, UnknownRJump_0xA8D6
0x0A8B4  F0 DB    main.asm:6627 ldh a, [$FFDB]
0x0A8B6  EA 37 AF main.asm:6628 ld [sObjectSpawnSprite], a
0x0A8B9  F0 DF    main.asm:6629 ldh a, [$FFDF]
0x0A8BB  EA C4 AF main.asm:6630 ld [sSavedFrameState], a
0x0A8BE  F0 E2    main.asm:6631 ldh a, [$FFE2]
0x0A8C0  EA CC AF main.asm:6632 ld [sSavedObjectType], a
0x0A8C3  FA C3 AF main.asm:6633 ld a, [sBossHitLatch]
0x0A8C6  A7       main.asm:6634 and a
0x0A8C7  CA 25 57 main.asm:6635 jp z, UnknownJump_0x9725
0x0A8CA  3E 52    main.asm:6636 ld a, 82
0x0A8CC  E0 DB    main.asm:6637 ldh [$FFDB], a
0x0A8CE  FA CD AF main.asm:6638 ld a, [sTimerReload]
0x0A8D1  E0 EC    main.asm:6639 ldh [$FFEC], a
0x0A8D3  C3 25 57 main.asm:6640 jp UnknownJump_0x9725
0x0A8D6  F0 DF    main.asm:6643 ldh a, [$FFDF]
0x0A8D8  A7       main.asm:6644 and a
0x0A8D9  20 2F    main.asm:6645 jr nz, UnknownRJump_0xA90A
0x0A8DB  FA 66 A2 main.asm:6646 ld a, [sFadeTimerLow]
0x0A8DE  FE 61    main.asm:6647 cp $61
0x0A8E0  CC FF 68 main.asm:6648 call z, UnknownCall_0xA8FF
0x0A8E3  FA CF AF main.asm:6649 ld a, [sEndingDoorActive]
0x0A8E6  A7       main.asm:6650 and a
0x0A8E7  C4 EF 72 main.asm:6651 call nz, UnknownCall_0xB2EF
0x0A8EA  CD 4E 71 main.asm:6652 call UnknownCall_0xB14E
0x0A8ED  C0       main.asm:6653 ret nz
0x0A8EE  AF       main.asm:6654 xor a
0x0A8EF  EA CF AF main.asm:6655 ld [sEndingDoorActive], a
0x0A8F2  3E 03    main.asm:6656 ld a, 3
0x0A8F4  E0 DF    main.asm:6657 ldh [$FFDF], a
0x0A8F6  3E 41    main.asm:6658 ld a, 65
0x0A8F8  E0 E2    main.asm:6659 ldh [$FFE2], a
0x0A8FA  3E 4F    main.asm:6660 ld a, 79
0x0A8FC  E0 DB    main.asm:6661 ldh [$FFDB], a
0x0A8FE  C9       main.asm:6662 ret
0x0A8FF  3E 0F    main.asm:6665 ld a, 15
0x0A901  EA 70 A4 main.asm:6666 ld [sSfxRequest1], a
0x0A904  3E 01    main.asm:6667 ld a, 1
0x0A906  EA CF AF main.asm:6668 ld [sEndingDoorActive], a
0x0A909  C9       main.asm:6669 ret
0x0A90A  F0 DF    main.asm:6672 ldh a, [$FFDF]
0x0A90C  3D       main.asm:6673 dec a
0x0A90D  EF       main.asm:6674 rst $28
0x0A920  CD 0E 74 main.asm:6681 call UnknownCall_0xB40E
0x0A923  CD 8F 71 main.asm:6682 call UnknownCall_0xB18F
0x0A926  CD 76 72 main.asm:6683 call UnknownCall_0xB276
0x0A929  F0 D9    main.asm:6684 ldh a, [$FFD9]
0x0A92B  FE 8A    main.asm:6685 cp $8A
0x0A92D  D8       main.asm:6686 ret c
0x0A92E  3E 06    main.asm:6687 ld a, 6
0x0A930  E0 DF    main.asm:6688 ldh [$FFDF], a
0x0A932  3E 80    main.asm:6689 ld a, 128
0x0A934  E0 EC    main.asm:6690 ldh [$FFEC], a
0x0A936  C9       main.asm:6691 ret
0x0A937  CD 0E 74 main.asm:6692 call UnknownCall_0xB40E
0x0A93A  F0 EC    main.asm:6693 ldh a, [$FFEC]
0x0A93C  3D       main.asm:6694 dec a
0x0A93D  E0 EC    main.asm:6695 ldh [$FFEC], a
0x0A93F  C0       main.asm:6696 ret nz
0x0A940  AF       main.asm:6697 xor a
0x0A941  E0 EA    main.asm:6698 ldh [$FFEA], a
0x0A943  E0 DE    main.asm:6699 ldh [$FFDE], a
0x0A945  3E 08    main.asm:6700 ld a, 8
0x0A947  E0 DF    main.asm:6701 ldh [$FFDF], a
0x0A949  3E 4C    main.asm:6702 ld a, 76
0x0A94B  E0 DB    main.asm:6703 ldh [$FFDB], a
0x0A94D  3E 20    main.asm:6704 ld a, 32
0x0A94F  E0 DD    main.asm:6705 ldh [$FFDD], a
0x0A951  C9       main.asm:6706 ret
0x0A952  FA D0 AF main.asm:6707 ld a, [sBossDelay]
0x0A955  A7       main.asm:6708 and a
0x0A956  C8       main.asm:6709 ret z
0x0A957  CD 21 73 main.asm:6710 call UnknownCall_0xB321
0x0A95A  F0 EC    main.asm:6711 ldh a, [$FFEC]
0x0A95C  3D       main.asm:6712 dec a
0x0A95D  E0 EC    main.asm:6713 ldh [$FFEC], a
0x0A95F  C0       main.asm:6714 ret nz
0x0A960  3E 0B    main.asm:6715 ld a, 11
0x0A962  E0 DF    main.asm:6716 ldh [$FFDF], a
0x0A964  3E 72    main.asm:6717 ld a, 114
0x0A966  E0 DB    main.asm:6718 ldh [$FFDB], a
0x0A968  AF       main.asm:6719 xor a
0x0A969  EA D0 AF main.asm:6720 ld [sBossDelay], a
0x0A96C  C9       main.asm:6721 ret
0x0A96D  3E 05    main.asm:6722 ld a, 5
0x0A96F  EA 66 A2 main.asm:6723 ld [sFadeTimerLow], a
0x0A972  CD 0E 74 main.asm:6724 call UnknownCall_0xB40E
0x0A975  CD 39 73 main.asm:6725 call UnknownCall_0xB339
0x0A978  06 02    main.asm:6726 ld b, 2
0x0A97A  CD 6A 72 main.asm:6727 call UnknownCall_0xB26A
0x0A97D  F0 DA    main.asm:6728 ldh a, [$FFDA]
0x0A97F  FE C0    main.asm:6729 cp $C0
0x0A981  D8       main.asm:6730 ret c
0x0A982  CD 5B 71 main.asm:6731 call UnknownCall_0xB15B
0x0A985  3E 01    main.asm:6732 ld a, 1
0x0A987  EA DC A2 main.asm:6733 ld [sBossFightPhase], a
0x0A98A  AF       main.asm:6734 xor a
0x0A98B  EA 8B A2 main.asm:6735 ld [sSceneState], a
0x0A98E  21 00 AB main.asm:6736 ld hl, sObjectStreamBuf
0x0A991  3E FF    main.asm:6737 ld a, 255
0x0A993  22       main.asm:6738 ld [hli], a
0x0A994  22       main.asm:6739 ld [hli], a
0x0A995  22       main.asm:6740 ld [hli], a
0x0A996  22       main.asm:6741 ld [hli], a
0x0A997  22       main.asm:6742 ld [hli], a
0x0A998  22       main.asm:6743 ld [hli], a
0x0A999  3E 07    main.asm:6744 ld a, 7
0x0A99B  22       main.asm:6745 ld [hli], a
0x0A99C  3E C8    main.asm:6746 ld a, 200
0x0A99E  22       main.asm:6747 ld [hli], a
0x0A99F  3E 00    main.asm:6748 ld a, 0
0x0A9A1  22       main.asm:6749 ld [hli], a
0x0A9A2  3E 00    main.asm:6750 ld a, 0
0x0A9A4  22       main.asm:6751 ld [hli], a
0x0A9A5  3E B0    main.asm:6752 ld a, 176
0x0A9A7  22       main.asm:6753 ld [hli], a
0x0A9A8  3E 1D    main.asm:6754 ld a, 29
0x0A9AA  22       main.asm:6755 ld [hli], a
0x0A9AB  36 FF    main.asm:6756 ld [hl], 255
0x0A9AD  3E AB    main.asm:6757 ld a, 171
0x0A9AF  EA 1E AF main.asm:6758 ld [sObjectStreamPtrHi], a
0x0A9B2  3E 06    main.asm:6759 ld a, 6
0x0A9B4  EA 1F AF main.asm:6760 ld [sObjectStreamPtrLo], a
0x0A9B7  3E 01    main.asm:6761 ld a, 1
0x0A9B9  EA 06 AF main.asm:6762 ld [sSpriteMapIndex], a
0x0A9BC  C9       main.asm:6763 ret
0x0A9BD  F0 E8    main.asm:6764 ldh a, [$FFE8]
0x0A9BF  A7       main.asm:6765 and a
0x0A9C0  20 12    main.asm:6766 jr nz, UnknownRJump_0xA9D4
0x0A9C2  CD E4 71 main.asm:6767 call UnknownCall_0xB1E4
0x0A9C5  CD 70 72 main.asm:6768 call UnknownCall_0xB270
0x0A9C8  F0 E0    main.asm:6769 ldh a, [$FFE0]
0x0A9CA  B9       main.asm:6770 cp c
0x0A9CB  C0       main.asm:6771 ret nz
0x0A9CC  AF       main.asm:6772 xor a
0x0A9CD  E0 E0    main.asm:6773 ldh [$FFE0], a
0x0A9CF  3E 01    main.asm:6774 ld a, 1
0x0A9D1  E0 E8    main.asm:6775 ldh [$FFE8], a
0x0A9D3  C9       main.asm:6776 ret
0x0A9D4  CD 7F 71 main.asm:6779 call UnknownCall_0xB17F
0x0A9D7  CD 76 72 main.asm:6780 call UnknownCall_0xB276
0x0A9DA  F0 D9    main.asm:6781 ldh a, [$FFD9]
0x0A9DC  FE 8A    main.asm:6782 cp $8A
0x0A9DE  D8       main.asm:6783 ret c
0x0A9DF  AF       main.asm:6784 xor a
0x0A9E0  E0 E0    main.asm:6785 ldh [$FFE0], a
0x0A9E2  E0 E8    main.asm:6786 ldh [$FFE8], a
0x0A9E4  3E 09    main.asm:6787 ld a, 9
0x0A9E6  E0 DF    main.asm:6788 ldh [$FFDF], a
0x0A9E8  3E 34    main.asm:6789 ld a, 52
0x0A9EA  EA B3 A2 main.asm:6790 ld [sCameraPanTimer], a
0x0A9ED  3E 01    main.asm:6791 ld a, 1
0x0A9EF  EA CE AF main.asm:6792 ld [sBossTransition], a
0x0A9F2  3E 18    main.asm:6793 ld a, 24
0x0A9F4  EA 60 A4 main.asm:6794 ld [sSfxRequest0], a
0x0A9F7  3E 30    main.asm:6795 ld a, 48
0x0A9F9  E0 EC    main.asm:6796 ldh [$FFEC], a
0x0A9FB  C9       main.asm:6797 ret
0x0A9FC  3E 05    main.asm:6800 ld a, 5
0x0A9FE  E0 DF    main.asm:6801 ldh [$FFDF], a
0x0AA00  3E 4F    main.asm:6802 ld a, 79
0x0AA02  E0 DB    main.asm:6803 ldh [$FFDB], a
0x0AA04  C9       main.asm:6804 ret
0x0AA05  FA 54 A2 main.asm:6805 ld a, [sTimerLow]
0x0AA08  E6 07    main.asm:6806 and $07
0x0AA0A  28 F0    main.asm:6807 jr z, UnknownRJump_0xA9FC
0x0AA0C  CD 39 73 main.asm:6808 call UnknownCall_0xB339
0x0AA0F  06 02    main.asm:6809 ld b, 2
0x0AA11  F0 E9    main.asm:6810 ldh a, [$FFE9]
0x0AA13  A7       main.asm:6811 and a
0x0AA14  20 0A    main.asm:6812 jr nz, UnknownRJump_0xAA20
0x0AA16  CD 64 72 main.asm:6813 call UnknownCall_0xB264
0x0AA19  F0 DA    main.asm:6814 ldh a, [$FFDA]
0x0AA1B  FE 10    main.asm:6815 cp $10
0x0AA1D  D0       main.asm:6816 ret nc
0x0AA1E  18 08    main.asm:6817 jr UnknownRJump_0xAA28
0x0AA20  CD 6A 72 main.asm:6820 call UnknownCall_0xB26A
0x0AA23  F0 DA    main.asm:6821 ldh a, [$FFDA]
0x0AA25  FE 98    main.asm:6822 cp $98
0x0AA27  D8       main.asm:6823 ret c
0x0AA28  F0 DD    main.asm:6826 ldh a, [$FFDD]
0x0AA2A  EE 20    main.asm:6827 xor $20
0x0AA2C  E0 DD    main.asm:6828 ldh [$FFDD], a
0x0AA2E  F0 E9    main.asm:6829 ldh a, [$FFE9]
0x0AA30  EE 01    main.asm:6830 xor $01
0x0AA32  E0 E9    main.asm:6831 ldh [$FFE9], a
0x0AA34  3E 08    main.asm:6832 ld a, 8
0x0AA36  E0 EC    main.asm:6833 ldh [$FFEC], a
0x0AA38  C9       main.asm:6834 ret
0x0AA39  F0 E8    main.asm:6835 ldh a, [$FFE8]
0x0AA3B  A7       main.asm:6836 and a
0x0AA3C  20 08    main.asm:6837 jr nz, UnknownRJump_0xAA46
0x0AA3E  CD D6 71 main.asm:6838 call UnknownCall_0xB1D6
0x0AA41  CD 70 72 main.asm:6839 call UnknownCall_0xB270
0x0AA44  18 06    main.asm:6840 jr UnknownRJump_0xAA4C
0x0AA46  CD 8F 71 main.asm:6843 call UnknownCall_0xB18F
0x0AA49  CD 76 72 main.asm:6844 call UnknownCall_0xB276
0x0AA4C  F0 E0    main.asm:6847 ldh a, [$FFE0]
0x0AA4E  B9       main.asm:6848 cp c
0x0AA4F  C0       main.asm:6849 ret nz
0x0AA50  AF       main.asm:6850 xor a
0x0AA51  E0 E0    main.asm:6851 ldh [$FFE0], a
0x0AA53  F0 E8    main.asm:6852 ldh a, [$FFE8]
0x0AA55  EE 01    main.asm:6853 xor $01
0x0AA57  E0 E8    main.asm:6854 ldh [$FFE8], a
0x0AA59  C0       main.asm:6855 ret nz
0x0AA5A  3E 09    main.asm:6856 ld a, 9
0x0AA5C  E0 DF    main.asm:6857 ldh [$FFDF], a
0x0AA5E  3E 38    main.asm:6858 ld a, 56
0x0AA60  EA B3 A2 main.asm:6859 ld [sCameraPanTimer], a
0x0AA63  E0 EC    main.asm:6860 ldh [$FFEC], a
0x0AA65  3E 01    main.asm:6861 ld a, 1
0x0AA67  EA CE AF main.asm:6862 ld [sBossTransition], a
0x0AA6A  3E 18    main.asm:6863 ld a, 24
0x0AA6C  EA 60 A4 main.asm:6864 ld [sSfxRequest0], a
0x0AA6F  C9       main.asm:6865 ret
0x0AA70  F0 EC    main.asm:6866 ldh a, [$FFEC]
0x0AA72  A7       main.asm:6867 and a
0x0AA73  28 04    main.asm:6868 jr z, UnknownRJump_0xAA79
0x0AA75  3D       main.asm:6869 dec a
0x0AA76  E0 EC    main.asm:6870 ldh [$FFEC], a
0x0AA78  C9       main.asm:6871 ret
0x0AA79  F0 E8    main.asm:6874 ldh a, [$FFE8]
0x0AA7B  A7       main.asm:6875 and a
0x0AA7C  20 08    main.asm:6876 jr nz, UnknownRJump_0xAA86
0x0AA7E  CD E4 71 main.asm:6877 call UnknownCall_0xB1E4
0x0AA81  CD 70 72 main.asm:6878 call UnknownCall_0xB270
0x0AA84  18 06    main.asm:6879 jr UnknownRJump_0xAA8C
0x0AA86  CD 83 71 main.asm:6882 call UnknownCall_0xB183
0x0AA89  CD 76 72 main.asm:6883 call UnknownCall_0xB276
0x0AA8C  F0 E0    main.asm:6886 ldh a, [$FFE0]
0x0AA8E  B9       main.asm:6887 cp c
0x0AA8F  C0       main.asm:6888 ret nz
0x0AA90  AF       main.asm:6889 xor a
0x0AA91  E0 E0    main.asm:6890 ldh [$FFE0], a
0x0AA93  E0 EA    main.asm:6891 ldh [$FFEA], a
0x0AA95  3E 4C    main.asm:6892 ld a, 76
0x0AA97  E0 DB    main.asm:6893 ldh [$FFDB], a
0x0AA99  F0 E8    main.asm:6894 ldh a, [$FFE8]
0x0AA9B  EE 01    main.asm:6895 xor $01
0x0AA9D  E0 E8    main.asm:6896 ldh [$FFE8], a
0x0AA9F  C0       main.asm:6897 ret nz
0x0AAA0  3E 04    main.asm:6898 ld a, 4
0x0AAA2  E0 DF    main.asm:6899 ldh [$FFDF], a
0x0AAA4  C9       main.asm:6900 ret
0x0B14E  FA 66 A2 main.asm:7780 ld a, [sFadeTimerLow]
0x0B151  47       main.asm:7781 ld b, a
0x0B152  FA 67 A2 main.asm:7782 ld a, [sFadeTimerHigh]
0x0B155  B0       main.asm:7783 or b
0x0B156  C9       main.asm:7784 ret
0x0B15B  21 D0 FF main.asm:7792 ld hl, $FFD0
0x0B15E  AF       main.asm:7793 xor a
0x0B15F  06 20    main.asm:7794 ld b, 32
0x0B161  22       main.asm:7797 ld [hli], a
0x0B162  05       main.asm:7798 dec b
0x0B163  20 FC    main.asm:7799 jr nz, UnknownRJump_0xB161
0x0B165  3E 01    main.asm:7800 ld a, 1
0x0B167  EA 3E AF main.asm:7801 ld [sObjectDone], a
0x0B16A  C9       main.asm:7802 ret
0x0B17F  0E 18    main.asm:7825 ld c, 24
0x0B181  18 0E    main.asm:7826 jr UnknownRJump_0xB191
0x0B183  0E 0C    main.asm:7829 ld c, 12
0x0B185  18 0A    main.asm:7830 jr UnknownRJump_0xB191
0x0B18F  0E 15    main.asm:7841 ld c, 21
0x0B191  21 A3 71 main.asm:7844 ld hl, $71A3
0x0B194  F0 E0    main.asm:7845 ldh a, [$FFE0]
0x0B196  5F       main.asm:7846 ld e, a
0x0B197  16 00    main.asm:7847 ld d, 0
0x0B199  19       main.asm:7848 add hl, de
0x0B19A  46       main.asm:7849 ld b, [hl]
0x0B19B  21 E0 FF main.asm:7850 ld hl, $FFE0
0x0B19E  7E       main.asm:7851 ld a, [hl]
0x0B19F  B9       main.asm:7852 cp c
0x0B1A0  C8       main.asm:7853 ret z
0x0B1A1  34       main.asm:7854 inc [hl]
0x0B1A2  C9       main.asm:7855 ret
0x0B1D6  0E 15    main.asm:7874 ld c, 21
0x0B1D8  21 0E 72 main.asm:7875 ld hl, $720E
0x0B1DB  18 13    main.asm:7876 jr UnknownRJump_0xB1F0
0x0B1E4  0E 0C    main.asm:7884 ld c, 12
0x0B1E6  21 17 72 main.asm:7885 ld hl, $7217
0x0B1E9  18 05    main.asm:7886 jr UnknownRJump_0xB1F0
0x0B1F0  F0 E0    main.asm:7893 ldh a, [$FFE0]
0x0B1F2  5F       main.asm:7894 ld e, a
0x0B1F3  16 00    main.asm:7895 ld d, 0
0x0B1F5  19       main.asm:7896 add hl, de
0x0B1F6  46       main.asm:7897 ld b, [hl]
0x0B1F7  21 E0 FF main.asm:7898 ld hl, $FFE0
0x0B1FA  7E       main.asm:7899 ld a, [hl]
0x0B1FB  B9       main.asm:7900 cp c
0x0B1FC  C8       main.asm:7901 ret z
0x0B1FD  34       main.asm:7902 inc [hl]
0x0B1FE  C9       main.asm:7903 ret
0x0B264  F0 DA    main.asm:7953 ldh a, [$FFDA]
0x0B266  90       main.asm:7954 sub b
0x0B267  E0 DA    main.asm:7955 ldh [$FFDA], a
0x0B269  C9       main.asm:7956 ret
0x0B26A  F0 DA    main.asm:7959 ldh a, [$FFDA]
0x0B26C  80       main.asm:7960 add b
0x0B26D  E0 DA    main.asm:7961 ldh [$FFDA], a
0x0B26F  C9       main.asm:7962 ret
0x0B270  F0 D9    main.asm:7965 ldh a, [$FFD9]
0x0B272  90       main.asm:7966 sub b
0x0B273  E0 D9    main.asm:7967 ldh [$FFD9], a
0x0B275  C9       main.asm:7968 ret
0x0B276  F0 D9    main.asm:7971 ldh a, [$FFD9]
0x0B278  80       main.asm:7972 add b
0x0B279  E0 D9    main.asm:7973 ldh [$FFD9], a
0x0B27B  C9       main.asm:7974 ret
0x0B2EF  F0 F0    main.asm:8059 ldh a, [$FFF0]
0x0B2F1  E6 07    main.asm:8060 and $07
0x0B2F3  C0       main.asm:8061 ret nz
0x0B2F4  18 0C    main.asm:8062 jr UnknownRJump_0xB302
0x0B302  F0 DB    main.asm:8076 ldh a, [$FFDB]
0x0B304  EE 01    main.asm:8077 xor $01
0x0B306  E0 DB    main.asm:8078 ldh [$FFDB], a
0x0B308  C9       main.asm:8079 ret
0x0B321  F0 F0    main.asm:8100 ldh a, [$FFF0]
0x0B323  E6 07    main.asm:8101 and $07
0x0B325  C0       main.asm:8102 ret nz
0x0B326  F0 DB    main.asm:8103 ldh a, [$FFDB]
0x0B328  EE 1F    main.asm:8104 xor $1F
0x0B32A  E0 DB    main.asm:8105 ldh [$FFDB], a
0x0B32C  C9       main.asm:8106 ret
0x0B339  F0 F0    main.asm:8118 ldh a, [$FFF0]
0x0B33B  E6 03    main.asm:8119 and $03
0x0B33D  C0       main.asm:8120 ret nz
0x0B33E  21 EA FF main.asm:8121 ld hl, $FFEA
0x0B341  34       main.asm:8122 inc [hl]
0x0B342  7E       main.asm:8123 ld a, [hl]
0x0B343  FE 04    main.asm:8124 cp $04
0x0B345  20 02    main.asm:8125 jr nz, UnknownRJump_0xB349
0x0B347  AF       main.asm:8126 xor a
0x0B348  77       main.asm:8127 ld [hl], a
0x0B349  21 56 73 main.asm:8130 ld hl, $7356
0x0B34C  5F       main.asm:8131 ld e, a
0x0B34D  16 00    main.asm:8132 ld d, 0
0x0B34F  19       main.asm:8133 add hl, de
0x0B350  F0 DB    main.asm:8134 ldh a, [$FFDB]
0x0B352  86       main.asm:8135 add [hl]
0x0B353  E0 DB    main.asm:8136 ldh [$FFDB], a
0x0B355  C9       main.asm:8137 ret
0x0B40E  F0 F0    main.asm:8282 ldh a, [$FFF0]
0x0B410  E6 03    main.asm:8283 and $03
0x0B412  C0       main.asm:8284 ret nz
0x0B413  F0 DE    main.asm:8285 ldh a, [$FFDE]
0x0B415  EE 10    main.asm:8286 xor $10
0x0B417  E0 DE    main.asm:8287 ldh [$FFDE], a
0x0B419  C9       main.asm:8288 ret
```

## Recovered rst $28 state tables (CPU targets)

- ROM 0x0A90E: $5CAC, $6920, $69BD, $6A05, $6A39, $6937, $6952, $696D, $6A70

## Existing helper dependencies supplied by the supervisor

- None.

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
