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

    public static byte[] Bytes(string text) => Encoding.ASCII.GetBytes(text);
}
