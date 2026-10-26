// The #244 surface: lenient reads, the header, validation, ifcXML and the
// reachability lint, as smoke.c's `capabilities()` checks them.

using System.Linq;
using Xunit;

namespace OpenBim.Ifc.Tests;

public class CapabilityTests
{
    [Fact]
    public void BeyondTheRecordModel()
    {
        var data = Fixtures.Bytes(Fixtures.Damaged);
        // docs:snippet dotnet-beyond-records
        // A damaged export: skip what cannot be read, and say what was skipped.
        using var model = IfcModel.Parse(data, ParseOptions.Lenient);
        var skipped = model.Diagnostics; // one message per recovery

        var header = model.Header; // Header { Name = ..., Author = [...], Schema = [...] }
        model.Header = header with { Author = new EquatableList<string>(new[] { "Reviewer" }) };

        var report = model.Validate(); // ValidationReport { Conformant = ..., Findings = [...] }
        var errors = report.Findings.Where(finding => finding.Severity == "error").ToList();

        var xml = model.WriteIfcXml(); // lossless ifcXML; or xsdProfile: "IFC4"
        using var fromXml = IfcModel.ParseIfcXml(xml);
        // docs:end

        Assert.Single(skipped);
        Assert.Equal(new[] { "Reviewer" }, model.Header.Author);
        Assert.NotEmpty(errors);
        Assert.Equal(2, fromXml.Count);
    }

    [Fact]
    public void AStrictReadRefusesWhatALenientOneSkips()
    {
        var data = Fixtures.Bytes(Fixtures.Damaged);
        Assert.Equal("parse", Assert.Throws<IfcException>(() => IfcModel.Parse(data)).Code);
        using var model = IfcModel.Parse(data, new ParseOptions(OnMalformed.Skip, CheckReferences: true));
        Assert.True(model.Diagnostics.Count >= 2, string.Join("\n", model.Diagnostics));
    }

    [Fact]
    public void TheHeaderIsReadAndReplacedWhole()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.Damaged), ParseOptions.Lenient);
        var header = model.Header;
        Assert.Equal("d.ifc", header.Name);
        Assert.Equal("2;1", header.ImplementationLevel);
        Assert.Equal(new[] { "Ann" }, header.Author);
        Assert.Equal(new[] { "IFC4" }, header.Schema);

        model.Header = header; // a no-op replacement is accepted
        Assert.Equal(header, model.Header);
        model.Header = header with { Name = "renamed.ifc" };
        using var again = IfcModel.Parse(model.Write());
        Assert.Equal("renamed.ifc", again.Header.Name);
    }

    [Fact]
    public void ValidationReportsTheDanglingReference()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.Damaged), ParseOptions.Lenient);
        var report = model.Validate();
        Assert.False(report.Conformant);
        Assert.False(report.Truncated);
        Assert.True(report.Errors >= 1);
        Assert.Equal(report.Errors + report.EvaluationErrors + report.Warnings + report.Unsupported, report.Findings.Count);
        Assert.Contains(report.Findings, finding => finding.Entity == 3UL && finding.Severity == "error");

        var capped = model.Validate(maxFindings: 1);
        Assert.Single(capped.Findings);
        Assert.Equal(new DanglingReference(3, 9), Assert.Single(model.DanglingReferences()));
    }

    [Fact]
    public void ExpressValidationRefusesTextThatDoesNotFitTheModel()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.File));
        var mismatch = Assert.Throws<IfcException>(() => model.ValidateExpress("SCHEMA IFC2X3; END_SCHEMA;"));
        Assert.Equal("express-mismatch", mismatch.Code);
        Assert.Equal(IfcStatus.ExpressMismatch, mismatch.Status);
        var invalid = Assert.Throws<IfcException>(() => model.ValidateExpress("SCHEMA IFC4; ENTITY ; END_SCHEMA;"));
        Assert.Equal("invalid-express", invalid.Code);
        Assert.Equal(IfcStatus.InvalidExpress, invalid.Status);
        Assert.Equal("invalid-express", Assert.Throws<IfcException>(() => model.ValidateExpress("")).Code);
        Assert.Throws<ArgumentOutOfRangeException>(() => model.ValidateExpress("x", maxFindings: 0));
        Assert.Equal(0, model.Validate().Indeterminate);
    }

    [Fact]
    public void IfcXmlRoundTripsAndRefusesAnUnknownProfile()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.File));
        using var fromXml = IfcModel.ParseIfcXml(model.WriteIfcXml());
        Assert.Equal(model.Attributes(1), fromXml.Attributes(1));
        Assert.Equal(model.Attributes(2), fromXml.Attributes(2));

        var error = Assert.Throws<IfcException>(() => model.WriteIfcXml("IFC2X3"));
        Assert.Equal("unsupported-profile", error.Code);
        Assert.Equal(IfcStatus.UnsupportedProfile, error.Status);
        Assert.Equal("unsupported-profile",
            Assert.Throws<IfcException>(() => IfcModel.ParseIfcXml(new byte[] { 0x3c }, "IFC2X3")).Code);
    }

    [Fact]
    public void IfcZipRoundTripsAndRefusesByCode()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.File));
        var zipped = model.WriteIfcZip();
        Assert.True(zipped.Length > 4 && zipped[0] == 0x50 && zipped[1] == 0x4b && zipped[2] == 0x03 && zipped[3] == 0x04);
        using var back = IfcModel.Parse(zipped);
        Assert.Equal(model.Write(), back.Write());
        using var fromXml = IfcModel.Parse(model.WriteIfcZip("ifcxml"));
        Assert.Equal(model.Attributes(1), fromXml.Attributes(1));

        Assert.Equal("invalid-value", Assert.Throws<IfcException>(() => model.WriteIfcZip("json")).Code);
        var empty = new byte[22];
        new byte[] { 0x50, 0x4b, 0x05, 0x06 }.CopyTo(empty, 0);
        Assert.Equal("parse", Assert.Throws<IfcException>(() => IfcModel.Parse(empty)).Code);
    }

    [Fact]
    public void AWallWithoutRepresentationIsNotUnreachable()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.File));
        Assert.Empty(model.UnreachableProducts());
    }
}
