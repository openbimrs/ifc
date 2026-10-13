using System.Text;
using Xunit;

// Leak checks compare IfcLibrary.LiveModels before and after a test, which
// only means something when no other test creates models meanwhile.
[assembly: CollectionBehavior(DisableTestParallelization = true)]

namespace OpenBim.Ifc.Tests;

/// <summary>The small files the suite reads, the same ones smoke.c uses.</summary>
internal static class Fixtures
{
    public const string File =
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n"
        + "FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\n"
        + "DATA;\n#1=IFCWALL('0abc',$,'Wall',*,$,$,$,$,.STANDARD.);\n"
        + "#2=IFCPROPERTYSINGLEVALUE('P',$,IFCLOGICAL(.U.),$);\nENDSEC;\n"
        + "END-ISO-10303-21;\n";

    /// <summary>A damaged export: #2 cannot be parsed, #3 names the missing #9.</summary>
    public const string Damaged =
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n"
        + "FILE_NAME('d.ifc','',('Ann'),(''),'','','');\nFILE_SCHEMA(('IFC4'));\n"
        + "ENDSEC;\nDATA;\n#1=IFCWALL('0abc',$,'Wall',$,$,$,$,$,.STANDARD.);\n"
        + "#2=IFCWALL('x',,;\n"
        + "#3=IFCRELDEFINESBYPROPERTIES('0def',$,$,$,(#1),#9);\nENDSEC;\n"
        + "END-ISO-10303-21;\n";

    /// <summary>A wall whose type holds a property set and a layer set (#123).</summary>
    public const string Domain =
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n"
        + "FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n"
        + "#2=IFCWALLTYPE('1YvctVUKr0kugbFTf53O9L',$,'WT',$,$,(#30),$,$,$,.SOLIDWALL.);\n"
        + "#3=IFCWALL('2YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,$,$,$,.STANDARD.);\n"
        + "#4=IFCRELDEFINESBYTYPE('3YvctVUKr0kugbFTf53O9L',$,$,$,(#3),#2);\n"
        + "#20=IFCMATERIAL('Concrete',$,$);\n"
        + "#22=IFCMATERIALLAYER(#20,0.2,.U.,'Core',$,$,$);\n"
        + "#24=IFCMATERIALLAYERSET((#22),'WT-200',$);\n"
        + "#26=IFCRELASSOCIATESMATERIAL('2ZvctVUKr0kugbFTf53O9L',$,$,$,(#2),#24);\n"
        + "#30=IFCPROPERTYSET('3ZvctVUKr0kugbFTf53O9L',$,'Pset_WallCommon',$,(#31));\n"
        + "#31=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);\n"
        + "ENDSEC;\nEND-ISO-10303-21;\n";

    public const string Ifc2x3 =
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n"
        + "FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC2X3'));\nENDSEC;\nDATA;\n"
        + "#1=IFCWALL('0abc',$,'Wall',$,$,$,$,$);\nENDSEC;\nEND-ISO-10303-21;\n";

    /// <summary>test/fixtures/synthetic-relations/relations_ifc2x3.ifc: #16 types #17 and #18; #17 is voided by #21 and #23; #21 is filled by #28 (#463).</summary>
    public const string RelationsIfc2x3 =
        "ISO-10303-21;\n"
        + "HEADER;\n"
        + "FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');\n"
        + "FILE_NAME('relations_ifc2x3.ifc','2026-10-13T00:00:00',('openbim.rs'),('openbim.rs'),'hand-authored','hand-authored','');\n"
        + "FILE_SCHEMA(('IFC2X3'));\n"
        + "ENDSEC;\n"
        + "DATA;\n"
        + "#1=IFCPERSON($,'Author',$,$,$,$,$,$);\n"
        + "#2=IFCORGANIZATION($,'openbim.rs',$,$,$);\n"
        + "#3=IFCPERSONANDORGANIZATION(#1,#2,$);\n"
        + "#4=IFCAPPLICATION(#2,'1','hand-authored','ha');\n"
        + "#5=IFCOWNERHISTORY(#3,#4,$,.ADDED.,$,$,$,0);\n"
        + "#6=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n"
        + "#7=IFCUNITASSIGNMENT((#6));\n"
        + "#8=IFCCARTESIANPOINT((0.,0.,0.));\n"
        + "#9=IFCAXIS2PLACEMENT3D(#8,$,$);\n"
        + "#10=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#9,$);\n"
        + "#11=IFCPROJECT('2lPwQcwif7Te2hEEhHdyr1',#5,'Relations',$,$,$,$,(#10),#7);\n"
        + "#12=IFCBUILDING('0cp5mlaK18ihYhPfA8qUdT',#5,'Building',$,$,$,$,$,.ELEMENT.,$,$,$);\n"
        + "#13=IFCBUILDINGSTOREY('0ZvF6NjNL2E8DV5UxGSMJJ',#5,'Level 0',$,$,$,$,$,.ELEMENT.,0.);\n"
        + "#14=IFCRELAGGREGATES('1tB5YgCJH1sAxRJ$z1EgUt',#5,$,$,#11,(#12));\n"
        + "#15=IFCRELAGGREGATES('3QH7FoJl5BN9hKbhDqwNz5',#5,$,$,#12,(#13));\n"
        + "#16=IFCWALLTYPE('1eQfQ_xBb9Be2k3wLJ3ymd',#5,'WT-200',$,$,$,$,$,$,.STANDARD.);\n"
        + "#17=IFCWALL('3KlXd8Mz13JRiyxcHC1k4c',#5,'Wall A',$,$,$,$,$);\n"
        + "#18=IFCWALL('0CZy29vG96FeYxekm5ZR8D',#5,'Wall B',$,$,$,$,$);\n"
        + "#19=IFCWALL('1GDj8VNpbB0g6aTma1DFgN',#5,'Wall C',$,$,$,$,$);\n"
        + "#20=IFCRELDEFINESBYTYPE('2DuaOrXNf5EuaCSJJZmNCY',#5,$,$,(#17,#18),#16);\n"
        + "#21=IFCOPENINGELEMENT('2OSHUSpd184xTJ1I8w5upd',#5,'Door opening',$,'OPENING',$,$,$);\n"
        + "#22=IFCOPENINGELEMENT('3jMqY2Lcf08hf5eiNVZtZR',#5,'Window opening',$,'OPENING',$,$,$);\n"
        + "#23=IFCOPENINGELEMENT('2CkO639TXFrfYxefxYPrFk',#5,'Recess',$,'RECESS',$,$,$);\n"
        + "#24=IFCRELVOIDSELEMENT('1sxldnraf9hOX0AYRNr__H',#5,$,$,#17,#21);\n"
        + "#25=IFCRELVOIDSELEMENT('0kcKFDecr6EBy$DVGINcuS',#5,$,$,#17,#23);\n"
        + "#26=IFCRELVOIDSELEMENT('06LnbbEQH04vreop4gsPBk',#5,$,$,#18,#22);\n"
        + "#27=IFCDOORSTYLE('3TjVg1vE954PVM8eBr37Vq',#5,'D-90',$,$,$,$,$,.SINGLE_SWING_LEFT.,.WOOD.,.F.,.F.);\n"
        + "#28=IFCDOOR('0j43avHLL3Ewlv_PjWp_$8',#5,'Door 1',$,$,$,$,$,2.1,0.9);\n"
        + "#29=IFCRELDEFINESBYTYPE('1LIMngr993cvee2DgWLKiF',#5,$,$,(#28),#27);\n"
        + "#30=IFCRELFILLSELEMENT('1bsCkDNjrAkhk9704yrSH9',#5,$,$,#21,#28);\n"
        + "#31=IFCWINDOW('0M9LtALEr5MBxJs3KQ2pV4',#5,'Window 1',$,$,$,$,$,1.2,1.);\n"
        + "#32=IFCRELFILLSELEMENT('2SuUBIzFPDDBkCiFVcwSHV',#5,$,$,#22,#31);\n"
        + "#33=IFCRELCONTAINEDINSPATIALSTRUCTURE('20pKXTj_z81x8FE$MvHNl3',#5,$,$,(#17,#18,#19,#28,#31),#13);\n"
        + "#34=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);\n"
        + "#35=IFCPROPERTYSET('0tASh_ntbBSQCHa9Qi_Uwt',#5,'Pset_WallCommon',$,(#34));\n"
        + "#36=IFCRELDEFINESBYPROPERTIES('3mMiS2Or91LeCkxV34$Xqg',#5,$,$,(#17),#35);\n"
        + "ENDSEC;\n"
        + "END-ISO-10303-21;\n";

    /// <summary>The same model in IFC4.</summary>
    public const string RelationsIfc4 =
        "ISO-10303-21;\n"
        + "HEADER;\n"
        + "FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');\n"
        + "FILE_NAME('relations_ifc4.ifc','2026-10-13T00:00:00',('openbim.rs'),('openbim.rs'),'hand-authored','hand-authored','');\n"
        + "FILE_SCHEMA(('IFC4'));\n"
        + "ENDSEC;\n"
        + "DATA;\n"
        + "#1=IFCPERSON($,'Author',$,$,$,$,$,$);\n"
        + "#2=IFCORGANIZATION($,'openbim.rs',$,$,$);\n"
        + "#3=IFCPERSONANDORGANIZATION(#1,#2,$);\n"
        + "#4=IFCAPPLICATION(#2,'1','hand-authored','ha');\n"
        + "#5=IFCOWNERHISTORY(#3,#4,$,.ADDED.,$,$,$,0);\n"
        + "#6=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n"
        + "#7=IFCUNITASSIGNMENT((#6));\n"
        + "#8=IFCCARTESIANPOINT((0.,0.,0.));\n"
        + "#9=IFCAXIS2PLACEMENT3D(#8,$,$);\n"
        + "#10=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#9,$);\n"
        + "#11=IFCPROJECT('2O0v8K0QvBEw4RtGyPLvD3',$,'Relations',$,$,$,$,(#10),#7);\n"
        + "#12=IFCBUILDING('2a_mntAY95uwR7vkTyku6c',$,'Building',$,$,$,$,$,.ELEMENT.,$,$,$);\n"
        + "#13=IFCBUILDINGSTOREY('03dmzL9eDElAM1RdSoTMq6',$,'Level 0',$,$,$,$,$,.ELEMENT.,0.);\n"
        + "#14=IFCRELAGGREGATES('1D6Y5hrJf3NQzrWTuWKwp6',$,$,$,#11,(#12));\n"
        + "#15=IFCRELAGGREGATES('1CJ4xjBJzAIgmFa3gjFaOS',$,$,$,#12,(#13));\n"
        + "#16=IFCWALLTYPE('0uEaQdwe14P8UQGe1lucJ9',$,'WT-200',$,$,$,$,$,$,.SOLIDWALL.);\n"
        + "#17=IFCWALL('0LwOtle8vEaxZPGHkxQdYT',$,'Wall A',$,$,$,$,$,.SOLIDWALL.);\n"
        + "#18=IFCWALL('1JneRqJiD7gAG9YRvV3_bR',$,'Wall B',$,$,$,$,$,.SOLIDWALL.);\n"
        + "#19=IFCWALL('3hWMyRWPXF79JFDAM4Ih2J',$,'Wall C',$,$,$,$,$,.SOLIDWALL.);\n"
        + "#20=IFCRELDEFINESBYTYPE('21QVwRmFbBYBWRS2$CRrfq',$,$,$,(#17,#18),#16);\n"
        + "#21=IFCOPENINGELEMENT('18mDylFCb3cQ_FHXSpwFDT',$,'Door opening',$,$,$,$,$,.OPENING.);\n"
        + "#22=IFCOPENINGELEMENT('2tdSYKMKbAi8A1cVGkK56A',$,'Window opening',$,$,$,$,$,.OPENING.);\n"
        + "#23=IFCOPENINGELEMENT('2xrGwuJev10gWOWE4qpi9z',$,'Recess',$,$,$,$,$,.RECESS.);\n"
        + "#24=IFCRELVOIDSELEMENT('0KgAr0B$T5zBImQKm8Ke07',$,$,$,#17,#21);\n"
        + "#25=IFCRELVOIDSELEMENT('15LtyKsPf0t9EmZfg3eues',$,$,$,#17,#23);\n"
        + "#26=IFCRELVOIDSELEMENT('3SuJuwjDf6eu00wMWPHLAh',$,$,$,#18,#22);\n"
        + "#27=IFCDOORTYPE('0aK0c6jXv1rg9duHEpyMLx',$,'D-90',$,$,$,$,$,$,.DOOR.,.SINGLE_SWING_LEFT.,.F.,$);\n"
        + "#28=IFCDOOR('0jxAppthz54uI93W0OK6X9',$,'Door 1',$,$,$,$,$,2.1,0.9,.DOOR.,.SINGLE_SWING_LEFT.,$);\n"
        + "#29=IFCRELDEFINESBYTYPE('3AU1k3RmDFFwrEDuQF_ANx',$,$,$,(#28),#27);\n"
        + "#30=IFCRELFILLSELEMENT('07BhqqlL50gxfeq9dhTnkO',$,$,$,#21,#28);\n"
        + "#31=IFCWINDOW('0QXjso$Dv4QODUII1Zefff',$,'Window 1',$,$,$,$,$,1.2,1.,.WINDOW.,.SINGLE_PANEL.,$);\n"
        + "#32=IFCRELFILLSELEMENT('1tPw3$YjrETh4EnK9cOfDT',$,$,$,#22,#31);\n"
        + "#33=IFCRELCONTAINEDINSPATIALSTRUCTURE('1ZmU8NAt5FdwIXVzcdRyVU',$,$,$,(#17,#18,#19,#28,#31),#13);\n"
        + "#34=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);\n"
        + "#35=IFCPROPERTYSET('2g3If28LX4auIiyQ5EEA_r',$,'Pset_WallCommon',$,(#34));\n"
        + "#36=IFCRELDEFINESBYPROPERTIES('0BibIyzuTEGhqntwrcY_Ht',$,$,$,(#17),#35);\n"
        + "ENDSEC;\n"
        + "END-ISO-10303-21;\n";

    /// <summary>The same model in IFC4X3.</summary>
    public const string RelationsIfc4x3 =
        "ISO-10303-21;\n"
        + "HEADER;\n"
        + "FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');\n"
        + "FILE_NAME('relations_ifc4x3.ifc','2026-10-13T00:00:00',('openbim.rs'),('openbim.rs'),'hand-authored','hand-authored','');\n"
        + "FILE_SCHEMA(('IFC4X3_ADD2'));\n"
        + "ENDSEC;\n"
        + "DATA;\n"
        + "#1=IFCPERSON($,'Author',$,$,$,$,$,$);\n"
        + "#2=IFCORGANIZATION($,'openbim.rs',$,$,$);\n"
        + "#3=IFCPERSONANDORGANIZATION(#1,#2,$);\n"
        + "#4=IFCAPPLICATION(#2,'1','hand-authored','ha');\n"
        + "#5=IFCOWNERHISTORY(#3,#4,$,.ADDED.,$,$,$,0);\n"
        + "#6=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n"
        + "#7=IFCUNITASSIGNMENT((#6));\n"
        + "#8=IFCCARTESIANPOINT((0.,0.,0.));\n"
        + "#9=IFCAXIS2PLACEMENT3D(#8,$,$);\n"
        + "#10=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#9,$);\n"
        + "#11=IFCPROJECT('2Hm0LIYGnDvwRBy2P2$XmF',$,'Relations',$,$,$,$,(#10),#7);\n"
        + "#12=IFCBUILDING('3I0vj7A4bE5vvU2aicSmj_',$,'Building',$,$,$,$,$,.ELEMENT.,$,$,$);\n"
        + "#13=IFCBUILDINGSTOREY('3gVZR4HaH4jAc4PioO8ylx',$,'Level 0',$,$,$,$,$,.ELEMENT.,0.);\n"
        + "#14=IFCRELAGGREGATES('0u$3doUZL2ve1TE6rAQDoL',$,$,$,#11,(#12));\n"
        + "#15=IFCRELAGGREGATES('3ThOD4qofFRu6yHiy9rwcN',$,$,$,#12,(#13));\n"
        + "#16=IFCWALLTYPE('1y3FT_sRvDAe66skM3AAat',$,'WT-200',$,$,$,$,$,$,.SOLIDWALL.);\n"
        + "#17=IFCWALL('26Du5_JO53uR6g_yd$zMyh',$,'Wall A',$,$,$,$,$,.SOLIDWALL.);\n"
        + "#18=IFCWALL('0SKYbzcYL7rfvGocJajC$2',$,'Wall B',$,$,$,$,$,.SOLIDWALL.);\n"
        + "#19=IFCWALL('3b5g9iTqTBT9Ab1yHy62JG',$,'Wall C',$,$,$,$,$,.SOLIDWALL.);\n"
        + "#20=IFCRELDEFINESBYTYPE('1KWFguX1DBPukvMMvcDR4v',$,$,$,(#17,#18),#16);\n"
        + "#21=IFCOPENINGELEMENT('1jVEA9Wkz35x6RfqtDsT_j',$,'Door opening',$,$,$,$,$,.OPENING.);\n"
        + "#22=IFCOPENINGELEMENT('0xIKCLGIL6_uqDTOug16Co',$,'Window opening',$,$,$,$,$,.OPENING.);\n"
        + "#23=IFCOPENINGELEMENT('0eShmHd0L0xROy$eWurFyk',$,'Recess',$,$,$,$,$,.RECESS.);\n"
        + "#24=IFCRELVOIDSELEMENT('2nZ6HRKvPDS93Xf4G9pRqk',$,$,$,#17,#21);\n"
        + "#25=IFCRELVOIDSELEMENT('0BMbD3xGX71PI2UGJl1RT0',$,$,$,#17,#23);\n"
        + "#26=IFCRELVOIDSELEMENT('1VgM$LaUn1xOyxOGXr6Qk8',$,$,$,#18,#22);\n"
        + "#27=IFCDOORTYPE('0THmndD310DhWZJ1sFXmnS',$,'D-90',$,$,$,$,$,$,.DOOR.,.SINGLE_SWING_LEFT.,.F.,$);\n"
        + "#28=IFCDOOR('1FHLF$GbjD0eBbXkCdfLxF',$,'Door 1',$,$,$,$,$,2.1,0.9,.DOOR.,.SINGLE_SWING_LEFT.,$);\n"
        + "#29=IFCRELDEFINESBYTYPE('12BdaeBzz1F8F2EVy4viDY',$,$,$,(#28),#27);\n"
        + "#30=IFCRELFILLSELEMENT('3UNuk8SivFZxc_FOjQRjEo',$,$,$,#21,#28);\n"
        + "#31=IFCWINDOW('1ytVO7tUf5BxUonU3KwC4T',$,'Window 1',$,$,$,$,$,1.2,1.,.WINDOW.,.SINGLE_PANEL.,$);\n"
        + "#32=IFCRELFILLSELEMENT('0JGmfPzV5Cw8XnFV$EXNcy',$,$,$,#22,#31);\n"
        + "#33=IFCRELCONTAINEDINSPATIALSTRUCTURE('3d6g2bFWX5TwXyZx5yXSJb',$,$,$,(#17,#18,#19,#28,#31),#13);\n"
        + "#34=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);\n"
        + "#35=IFCPROPERTYSET('2GuV0_R5TBOgYz_yXCVi6t',$,'Pset_WallCommon',$,(#34));\n"
        + "#36=IFCRELDEFINESBYPROPERTIES('1FgsaRjFf6gAaTzodyA0hu',$,$,$,(#17),#35);\n"
        + "ENDSEC;\n"
        + "END-ISO-10303-21;\n";

    public static byte[] Bytes(string text) => Encoding.ASCII.GetBytes(text);
}
