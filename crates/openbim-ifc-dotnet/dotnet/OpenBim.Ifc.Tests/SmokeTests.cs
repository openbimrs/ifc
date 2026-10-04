// The C smoke test (crates/openbim-ifc-capi/tests/c/smoke.c) from C#:
// parse, read, edit, add, write, re-parse, open from a path, and the error
// codes, all through the packed OpenBim.Ifc package.

using System;
using System.IO;
using System.Linq;
using System.Text;
using Xunit;

namespace OpenBim.Ifc.Tests;

public class SmokeTests
{
    [Fact]
    public void TheLibraryReportsAbi01()
    {
        var version = IfcLibrary.Version;
        Assert.Equal(0, version.Abi.Major);
        Assert.Equal(1, version.Abi.Minor);
        // Writing property sets arrived with ABI 0.1.3.
        Assert.True(version.Abi.Build >= 3, $"ABI {version.Abi}");
    }

    [Fact]
    public void ReadEditWrite()
    {
        var data = Fixtures.Bytes(Fixtures.File);
        // docs:snippet dotnet-read-edit-write
        using var model = IfcModel.Parse(data); // or IfcModel.Open("model.ifc")
        var schema = model.Schema; // "IFC4"

        foreach (var wall in model.IdsOfType("IfcWall"))
        {
            var name = (Value.Text)model.Attribute(wall, 2); // Text { Value = Wall }
            model.SetAttribute(wall, 2, new Value.Text(name.Value + " (checked)"));
        }

        byte[] written = model.Write(); // STEP, ready to save
        // docs:end

        Assert.Equal("IFC4", schema);
        using var again = IfcModel.Parse(written);
        Assert.Equal(new Value.Text("Wall (checked)"), again.Attribute(1, 2));
    }

    [Fact]
    public void KindsAHostCouldConfuseStayDistinct()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.File));
        Assert.Equal(2, model.Count);
        Assert.Equal(new ulong[] { 1, 2 }, model.Ids());
        Assert.Equal("IFCWALL", model.TypeOf(1));

        Assert.IsType<Value.Derived>(model.Attribute(1, 3));
        Assert.IsType<Value.Null>(model.Attribute(1, 4));
        Assert.Equal(new Value.Enum("STANDARD"), model.Attribute(1, 8));
        Assert.Equal(new Value.Typed("IFCLOGICAL", new Value.Unknown()), model.Attribute(2, 2));
        Assert.NotEqual<Value>(new Value.Integer(1), new Value.Real(1.0));
        Assert.NotEqual<Value>(new Value.Bool(false), new Value.Unknown());
        Assert.IsType<Value.Null>(model.Attribute(1, 40));

        var attributes = model.Attributes(1);
        Assert.Equal(9, attributes.Count);
        Assert.Equal(new Value.Text("0abc"), attributes[0]);
    }

    [Fact]
    public void EditsAndAdditionsSurviveAWrite()
    {
        byte[] written;
        using (var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.File)))
        {
            Assert.Equal(new Value.Text("Wall"), model.SetAttribute(1, 2, new Value.Text("Renamed")));
            // A `*` into a `$` slot exercises the decoder, which reading never reaches.
            model.SetAttribute(1, 4, new Value.Derived());
            var point = model.Add("IfcCartesianPoint", new Value[] { new Value.List(new Value.Real(1.5), new Value.Real(-2.0)) });
            Assert.Equal(3UL, point);
            written = model.Write();
        }

        using var again = IfcModel.Parse(written);
        Assert.Equal(new Value.Text("Renamed"), again.Attribute(1, 2));
        Assert.IsType<Value.Derived>(again.Attribute(1, 4));
        Assert.Equal(new Value.List(new Value.Real(1.5), new Value.Real(-2.0)), again.Attribute(3, 0));
        Assert.Equal("IFCCARTESIANPOINT", again.TypeOf(3));

        again.Remove(3);
        Assert.Equal(2, again.Count);
        Assert.Empty(again.DanglingReferences());
    }

    [Fact]
    public void TextIsUtf8BothWays()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.File));
        model.SetAttribute(1, 2, new Value.Text("Wand Ä – 壁"));
        using var again = IfcModel.Parse(model.Write());
        Assert.Equal(new Value.Text("Wand Ä – 壁"), again.Attribute(1, 2));
    }

    [Fact]
    public void ErrorsCarryTheSharedCodeAndTheCStatus()
    {
        var data = Fixtures.Bytes(Fixtures.File);
        // docs:snippet dotnet-errors
        using var model = IfcModel.Parse(data);
        try
        {
            model.Remove(99);
        }
        catch (IfcException error) when (error.Code == "missing-entity")
        {
            // error.Status == IfcStatus.MissingEntity; error.Message names #99
        }
        // docs:end

        var missing = Assert.Throws<IfcException>(() => model.TypeOf(99));
        Assert.Equal("missing-entity", missing.Code);
        Assert.Equal(IfcStatus.MissingEntity, missing.Status);
        Assert.Contains("99", missing.Message);

        var parse = Assert.Throws<IfcException>(() => IfcModel.Parse(Encoding.ASCII.GetBytes("not a STEP file")));
        Assert.Equal("parse", parse.Code);
        Assert.Equal(IfcStatus.Parse, parse.Status);
        Assert.False(string.IsNullOrEmpty(parse.Message));

        var real = Assert.Throws<IfcException>(() => model.SetAttribute(1, 2, new Value.Real(double.NaN)));
        Assert.Equal("invalid-value", real.Code);
        Assert.Equal(new Value.Text("Wall"), model.Attribute(1, 2));
    }

    [Fact]
    public void OpeningFromAPathReadsTheSameFile()
    {
        var path = Path.Combine(Path.GetTempPath(), "openbim_ifc_dotnet_" + Guid.NewGuid().ToString("N") + ".ifc");
        File.WriteAllText(path, Fixtures.File);
        try
        {
            using (var opened = IfcModel.Open(path))
            {
                Assert.Equal(2, opened.Count);
            }
            using (var mapped = IfcModel.Open(path, mapped: true))
            {
                Assert.Equal(2, mapped.Count);
            }
        }
        finally
        {
            File.Delete(path);
        }

        var io = Assert.Throws<IfcException>(() => IfcModel.Open(Path.Combine(Path.GetTempPath(), "openbim_ifc_dotnet_missing", "x.ifc")));
        Assert.Equal("io", io.Code);
        Assert.Equal(IfcStatus.Io, io.Status);
    }

    [Fact]
    public void ADisposedModelIsReleasedOnceAndRefusesCalls()
    {
        var before = IfcLibrary.LiveModels;
        var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.File));
        var empty = new IfcModel();
        Assert.Equal(before + 2, IfcLibrary.LiveModels);
        Assert.Equal(0, empty.Count);
        Assert.Null(empty.Schema);

        model.Dispose();
        model.Dispose();
        empty.Dispose();
        Assert.True(model.IsDisposed);
        Assert.Throws<ObjectDisposedException>(() => model.Count);
        Assert.Equal(before, IfcLibrary.LiveModels);
    }

    [Fact]
    public void AnUnreferencedModelIsFinalized()
    {
        var before = IfcLibrary.LiveModels;
        Leak();
        GC.Collect();
        GC.WaitForPendingFinalizers();
        GC.Collect();
        Assert.Equal(before, IfcLibrary.LiveModels);

        static void Leak() => _ = IfcModel.Parse(Fixtures.Bytes(Fixtures.File)).Count;
    }

    [Fact]
    public void ValuesCompareAndPrintByValue()
    {
        Assert.Equal(new Value.List(new Value.Ref(1), new Value.Text("x")), new Value.List(new Value.Ref(1), new Value.Text("x")));
        Assert.Equal("(#1,'it''s',.T.,$,*,.U.,IFCLABEL('x'))",
            new Value.List(new Value.Ref(1), new Value.Text("it's"), new Value.Bool(true), new Value.Null(),
                new Value.Derived(), new Value.Unknown(), new Value.Typed("IFCLABEL", new Value.Text("x"))).ToString());
        Assert.Equal(new[] { 1UL, 2UL }, new EquatableList<ulong>(new[] { 1UL, 2UL }).ToArray());
    }
}
