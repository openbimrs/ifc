BEGIN{
  q=sprintf("%c",39);
  print "ISO-10303-21;";
  print "HEADER;";
  print "FILE_DESCRIPTION(("q q"),"q"2;1"q");";
  print "FILE_NAME("q"bench.ifc"q","q"2026-01-01T00:00:00"q",("q q"),("q q"),"q q","q q","q q");";
  print "FILE_SCHEMA(("q"IFC4"q"));";
  print "ENDSEC;";
  print "DATA;";
  print "#1=IFCPERSON($,"q"A"q",$,$,$,$,$,$);";
  print "#2=IFCORGANIZATION($,"q"O"q",$,$,$);";
  print "#3=IFCPERSONANDORGANIZATION(#1,#2,$);";
  print "#4=IFCAPPLICATION(#2,"q"1"q","q"app"q","q"app"q");";
  print "#5=IFCOWNERHISTORY(#3,#4,$,.ADDED.,$,$,$,0);";
  print "#6=IFCCARTESIANPOINT((0.,0.,0.));";
  print "#7=IFCAXIS2PLACEMENT3D(#6,$,$);";
  print "#8=IFCLOCALPLACEMENT($,#7);";
  id=9;
  for(i=0;i<n;i++){
    pt=id; printf "#%d=IFCCARTESIANPOINT((%d.,%d.,0.));\n", id, i*3, i*2; id++;
    ax=id; printf "#%d=IFCAXIS2PLACEMENT3D(#%d,$,$);\n", id, pt; id++;
    lp=id; printf "#%d=IFCLOCALPLACEMENT(#8,#%d);\n", id, ax; id++;
    pl=id; printf "#%d=IFCPOLYLINE((#%d,#%d));\n", id, pt, pt; id++;
    sr=id; printf "#%d=IFCSHAPEREPRESENTATION($,"q"Axis"q","q"Curve2D"q",(#%d));\n", id, pl; id++;
    pd=id; printf "#%d=IFCPRODUCTDEFINITIONSHAPE($,$,(#%d));\n", id, sr; id++;
    w=id; printf "#%d=IFCWALL("q"%022d"q",#5,"q"W%d"q",$,$,#%d,#%d,$,$);\n", id, i, i, lp, pd; id++;
    pv=id; printf "#%d=IFCPROPERTYSINGLEVALUE("q"P"q",$,IFCLABEL("q"v%d"q"),$);\n", id, i; id++;
    ps=id; printf "#%d=IFCPROPERTYSET("q"%022d"q",#5,"q"PS"q",$,(#%d));\n", id, i+1000000, pv; id++;
    printf "#%d=IFCRELDEFINESBYPROPERTIES("q"%022d"q",#5,$,$,(#%d),#%d);\n", id, i+2000000, w, ps; id++;
  }
  print "ENDSEC;";
  print "END-ISO-10303-21;";
}
